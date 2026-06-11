//! Answer pipeline: pack retrieved chunks into a token-budgeted context, build
//! a prompt that forces inline `[^chunk_id]` citations, and post-process the
//! model's output to keep only citations that point at chunks we actually
//! supplied (the anti-hallucination guard).

pub mod llm;

use crate::ingest::chunker::estimate_tokens;
use crate::retrieve::RetrievedChunk;

/// Default context budget in tokens (chunks only, excludes the prompt scaffold).
pub const DEFAULT_CONTEXT_BUDGET: usize = 6000;

/// If the best retrieved chunk scores below this, instruct the model to decline
/// rather than answer from thin context.
pub const DEFAULT_REJECT_THRESHOLD: f32 = 0.15;

/// Greedily pack the highest-scoring chunks into the token budget. Input is
/// assumed sorted best-first. Returns the selected chunks and the assembled
/// context block (each chunk labeled with its `[^chunk_id]` and source path).
pub fn pack_context(chunks: &[RetrievedChunk], budget: usize) -> (Vec<RetrievedChunk>, String) {
    let mut selected = Vec::new();
    let mut context = String::new();
    let mut used = 0;

    for chunk in chunks {
        let cost = estimate_tokens(&chunk.text);
        // Always take at least one chunk so a single large chunk still answers.
        if used + cost > budget && !selected.is_empty() {
            break;
        }
        used += cost;
        context.push_str(&format!(
            "[^{}] (source: {}):\n{}\n\n",
            chunk.chunk_id, chunk.path_or_url, chunk.text
        ));
        selected.push(chunk.clone());
    }

    (selected, context)
}

/// True if retrieval was too weak to answer (top score below threshold, or no
/// hits at all).
pub fn should_reject(chunks: &[RetrievedChunk], threshold: f32) -> bool {
    match chunks.first() {
        None => true,
        Some(top) => top.score < threshold,
    }
}

/// The system prompt: grounding rules + the citation contract.
pub fn system_prompt() -> &'static str {
    "You are Mnemos, a retrieval-grounded assistant. Answer ONLY from the \
provided context. After each claim, cite the supporting chunk inline using the \
exact marker [^chunk_id] with the id given in the context. Do not invent ids. \
If the context is insufficient, say you don't have enough information and list \
what you'd need — do not guess and do not fabricate citations."
}

/// Build the user prompt from the question and packed context.
pub fn build_prompt(query: &str, context: &str) -> String {
    format!(
        "Context:\n{context}\n---\nQuestion: {query}\n\nAnswer using only the \
context above, with inline [^chunk_id] citations after each supported claim."
    )
}

/// Extract cited chunk ids from `answer` in first-seen order, keeping only ids
/// present in `valid_ids`. Unknown ids (hallucinated citations) are dropped.
pub fn parse_citations(answer: &str, valid_ids: &[String]) -> Vec<String> {
    let valid: std::collections::HashSet<&str> = valid_ids.iter().map(|s| s.as_str()).collect();
    let mut out = Vec::new();
    let mut seen = std::collections::HashSet::new();
    let bytes = answer.as_bytes();
    let mut i = 0;
    while i + 1 < bytes.len() {
        if bytes[i] == b'[' && bytes[i + 1] == b'^' {
            if let Some(close) = answer[i + 2..].find(']') {
                let id = &answer[i + 2..i + 2 + close];
                if valid.contains(id) && seen.insert(id.to_string()) {
                    out.push(id.to_string());
                }
                i = i + 2 + close + 1;
                continue;
            }
        }
        i += 1;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Locator;

    fn chunk(id: &str, text: &str, score: f32) -> RetrievedChunk {
        RetrievedChunk {
            chunk_id: id.to_string(),
            text: text.to_string(),
            structural_path: None,
            locator: serde_json::to_string(&Locator::charspan(0, text.len())).unwrap(),
            path_or_url: format!("/docs/{id}.md"),
            score,
        }
    }

    #[test]
    fn pack_context_respects_budget_but_takes_at_least_one() {
        let big = "word ".repeat(4000); // ~4000 tokens
        let chunks = vec![chunk("a", &big, 0.9), chunk("b", &big, 0.8)];
        let (selected, ctx) = pack_context(&chunks, 1000);
        assert_eq!(
            selected.len(),
            1,
            "one oversized chunk should still be taken"
        );
        assert!(ctx.contains("[^a]"));
    }

    #[test]
    fn pack_context_includes_multiple_when_they_fit() {
        let chunks = vec![
            chunk("a", "short text one", 0.9),
            chunk("b", "short text two", 0.8),
        ];
        let (selected, ctx) = pack_context(&chunks, 6000);
        assert_eq!(selected.len(), 2);
        assert!(ctx.contains("[^a]") && ctx.contains("[^b]"));
    }

    #[test]
    fn should_reject_on_empty_or_low_score() {
        assert!(should_reject(&[], 0.15));
        assert!(should_reject(&[chunk("a", "x", 0.05)], 0.15));
        assert!(!should_reject(&[chunk("a", "x", 0.5)], 0.15));
    }

    #[test]
    fn parse_citations_keeps_valid_drops_unknown_and_dedups() {
        let valid = vec!["c1".to_string(), "c2".to_string()];
        let answer = "Claim one [^c1]. Claim two [^c2]. Repeat [^c1]. Fake [^nope].";
        let cites = parse_citations(answer, &valid);
        assert_eq!(cites, vec!["c1".to_string(), "c2".to_string()]);
    }

    #[test]
    fn parse_citations_handles_no_markers() {
        let valid = vec!["c1".to_string()];
        assert!(parse_citations("no citations here", &valid).is_empty());
    }

    #[test]
    fn prompt_contains_question_and_context() {
        let p = build_prompt("What is X?", "[^c1] (source: a): X is a thing.");
        assert!(p.contains("What is X?"));
        assert!(p.contains("[^c1]"));
    }
}
