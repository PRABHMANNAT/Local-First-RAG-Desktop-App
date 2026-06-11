//! Chat commands. M1 ships `search` (retrieval only), which the chat surface
//! uses to show grounded results with citation links. The streaming `ask`
//! command with an LLM-composed answer lands with the answer pipeline.

use serde::Serialize;
use serde_json::Value;
use tauri::State;

use crate::answer::{
    build_prompt, pack_context, parse_citations, should_reject, system_prompt,
    DEFAULT_CONTEXT_BUDGET, DEFAULT_REJECT_THRESHOLD,
};
use crate::db::repo;
use crate::error::AppResult;
use crate::retrieve::retrieve;
use crate::state::AppState;

fn now_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

/// A retrieved chunk shaped for the frontend. `locator` is the parsed JSON
/// object (not a string) so the citation drawer can switch on `kind`.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RetrievedChunkDto {
    pub chunk_id: String,
    pub text: String,
    pub structural_path: Option<String>,
    pub locator: Value,
    pub path_or_url: String,
    pub score: f32,
}

/// Retrieve the top-`k` chunks relevant to `query` from the active workspace.
#[tauri::command]
pub async fn search(
    state: State<'_, AppState>,
    query: String,
    k: usize,
) -> AppResult<Vec<RetrievedChunkDto>> {
    let ws = state.require_active().await?;
    let hits = retrieve(
        &ws.pool,
        ws.store.as_ref(),
        ws.embedder.as_ref(),
        &query,
        k.clamp(1, 50),
    )
    .await?;

    Ok(hits
        .into_iter()
        .map(|h| RetrievedChunkDto {
            locator: serde_json::from_str(&h.locator).unwrap_or(Value::Null),
            chunk_id: h.chunk_id,
            text: h.text,
            structural_path: h.structural_path,
            path_or_url: h.path_or_url,
            score: h.score,
        })
        .collect())
}

/// A citation returned with an answer: the chunk, whether the model actually
/// cited it inline, and its retrieval score.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CitationDto {
    pub chunk_id: String,
    pub text: String,
    pub structural_path: Option<String>,
    pub locator: Value,
    pub path_or_url: String,
    pub score: f32,
    pub used_in_answer: bool,
}

/// The result of asking a question.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AnswerDto {
    pub conversation_id: String,
    pub answer: String,
    pub citations: Vec<CitationDto>,
    /// True when retrieval was too weak and the model was asked to decline.
    pub rejected: bool,
}

/// Ask a question against the active workspace: retrieve, pack a budgeted
/// context, generate a cited answer, and persist the conversation + citations.
#[tauri::command]
pub async fn ask(state: State<'_, AppState>, query: String) -> AppResult<AnswerDto> {
    let ws = state.require_active().await?;
    let hits = retrieve(
        &ws.pool,
        ws.store.as_ref(),
        ws.embedder.as_ref(),
        &query,
        12,
    )
    .await?;

    let conversation_id = uuid::Uuid::now_v7().to_string();
    let ts = now_ms();
    repo::insert_conversation(&ws.pool, &conversation_id, Some(&truncate(&query, 60)), ts).await?;
    let user_msg = uuid::Uuid::now_v7().to_string();
    repo::insert_message(&ws.pool, &user_msg, &conversation_id, "user", &query, ts).await?;

    // Reject-to-answer: refuse rather than hallucinate from thin context.
    if should_reject(&hits, DEFAULT_REJECT_THRESHOLD) {
        let answer = "I don't have enough information in the indexed sources to \
answer that. Try adding a relevant source, or rephrase the question."
            .to_string();
        let asst = uuid::Uuid::now_v7().to_string();
        repo::insert_message(
            &ws.pool,
            &asst,
            &conversation_id,
            "assistant",
            &answer,
            now_ms(),
        )
        .await?;
        return Ok(AnswerDto {
            conversation_id,
            answer,
            citations: Vec::new(),
            rejected: true,
        });
    }

    let (selected, context) = pack_context(&hits, DEFAULT_CONTEXT_BUDGET);
    let prompt = build_prompt(&query, &context);
    let raw = ws.llm.generate(system_prompt(), &prompt).await?;

    let valid_ids: Vec<String> = selected.iter().map(|c| c.chunk_id.clone()).collect();
    let cited = parse_citations(&raw, &valid_ids);

    let asst = uuid::Uuid::now_v7().to_string();
    repo::insert_message(
        &ws.pool,
        &asst,
        &conversation_id,
        "assistant",
        &raw,
        now_ms(),
    )
    .await?;

    let mut citations = Vec::with_capacity(selected.len());
    for c in selected {
        let used = cited.contains(&c.chunk_id);
        repo::insert_citation(&ws.pool, &asst, &c.chunk_id, c.score as f64, used).await?;
        citations.push(CitationDto {
            locator: serde_json::from_str(&c.locator).unwrap_or(Value::Null),
            chunk_id: c.chunk_id,
            text: c.text,
            structural_path: c.structural_path,
            path_or_url: c.path_or_url,
            score: c.score,
            used_in_answer: used,
        });
    }

    Ok(AnswerDto {
        conversation_id,
        answer: raw,
        citations,
        rejected: false,
    })
}

/// Truncate a string to `max` chars for use as a conversation title.
fn truncate(s: &str, max: usize) -> String {
    let t = s.trim();
    if t.chars().count() <= max {
        t.to_string()
    } else {
        let cut: String = t.chars().take(max).collect();
        format!("{cut}…")
    }
}
