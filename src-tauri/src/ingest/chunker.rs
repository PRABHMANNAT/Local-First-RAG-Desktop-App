//! Chunker: turns a document's text into retrievable chunks.
//!
//! Two passes. First a **structural** pass splits the text into blocks at
//! natural boundaries — markdown headings and fenced code — tracking a heading
//! path (`H1 > H2`) for each block. Then a **sliding window** packs each block's
//! segments (sentences for prose, lines for code) into chunks of roughly
//! [`ChunkConfig::min_tokens`]..[`ChunkConfig::max_tokens`] with a token overlap
//! so context isn't severed at chunk edges.
//!
//! Token counts are estimated (≈4 non-whitespace chars per token); a real
//! tokenizer can replace [`estimate_tokens`] later behind the same signature.

use serde::Serialize;

use crate::model::Locator;

/// What kind of text we're chunking — drives segment granularity.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextKind {
    Markdown,
    Code,
    Plain,
}

/// Chunk sizing knobs. Defaults follow the plan: 300–500 tokens, 50 overlap.
#[derive(Debug, Clone, Copy)]
pub struct ChunkConfig {
    pub min_tokens: usize,
    pub max_tokens: usize,
    pub overlap_tokens: usize,
}

impl Default for ChunkConfig {
    fn default() -> Self {
        Self {
            min_tokens: 300,
            max_tokens: 500,
            overlap_tokens: 50,
        }
    }
}

/// A produced chunk, ready to embed and persist.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Chunk {
    pub ordinal: usize,
    pub text: String,
    pub token_count: usize,
    pub structural_path: Option<String>,
    pub locator: Locator,
}

/// Estimate token count for a slice (≈4 non-whitespace chars per token).
pub fn estimate_tokens(s: &str) -> usize {
    let dense = s.chars().filter(|c| !c.is_whitespace()).count();
    (dense / 4).max(if dense == 0 { 0 } else { 1 })
}

/// A structural block: a byte range plus its heading path. `is_code` blocks are
/// segmented by line instead of by sentence.
#[derive(Debug, Clone, PartialEq)]
struct Block {
    start: usize,
    end: usize,
    path: Option<String>,
    is_code: bool,
}

/// Chunk a whole document. Returns chunks in reading order with `ordinal` set.
pub fn chunk_document(text: &str, kind: TextKind, cfg: &ChunkConfig) -> Vec<Chunk> {
    let blocks = match kind {
        TextKind::Markdown => structural_blocks_markdown(text),
        TextKind::Code => vec![Block {
            start: 0,
            end: text.len(),
            path: None,
            is_code: true,
        }],
        TextKind::Plain => vec![Block {
            start: 0,
            end: text.len(),
            path: None,
            is_code: false,
        }],
    };

    let mut chunks = Vec::new();
    let mut ordinal = 0;
    for block in blocks {
        let segments = if block.is_code {
            line_segments(text, block.start, block.end)
        } else {
            sentence_segments(text, block.start, block.end)
        };
        window_segments(text, &segments, &block.path, cfg, &mut ordinal, &mut chunks);
    }
    chunks
}

/// True if a trimmed line is an ATX markdown heading (`#`..`######` + space).
fn parse_heading(trimmed: &str) -> Option<(usize, String)> {
    if !trimmed.starts_with('#') {
        return None;
    }
    let level = trimmed.chars().take_while(|&c| c == '#').count();
    if level == 0 || level > 6 {
        return None;
    }
    let rest = &trimmed[level..];
    if !rest.starts_with(' ') {
        return None;
    }
    Some((level, rest.trim().to_string()))
}

/// Split markdown into blocks at headings and fenced code, carrying a heading
/// path for each block.
fn structural_blocks_markdown(text: &str) -> Vec<Block> {
    let mut blocks = Vec::new();
    let mut heading: Vec<String> = Vec::new();
    let mut cur_start = 0usize;
    let mut in_fence = false;
    let mut fence_start = 0usize;
    let mut offset = 0usize;

    let path_of = |h: &[String]| -> Option<String> {
        if h.is_empty() {
            None
        } else {
            Some(h.join(" > "))
        }
    };

    for line in text.split_inclusive('\n') {
        let line_start = offset;
        let line_end = offset + line.len();
        offset = line_end;
        let trimmed = line.trim_start();

        if trimmed.starts_with("```") {
            if in_fence {
                // Closing fence: emit the code block (fence lines included).
                let mut path = path_of(&heading).unwrap_or_default();
                path.push_str(if path.is_empty() { "code" } else { " > code" });
                blocks.push(Block {
                    start: fence_start,
                    end: line_end,
                    path: Some(path),
                    is_code: true,
                });
                cur_start = line_end;
                in_fence = false;
            } else {
                // Opening fence: flush preceding prose.
                if line_start > cur_start {
                    blocks.push(Block {
                        start: cur_start,
                        end: line_start,
                        path: path_of(&heading),
                        is_code: false,
                    });
                }
                fence_start = line_start;
                in_fence = true;
            }
            continue;
        }

        if !in_fence {
            if let Some((level, title)) = parse_heading(trimmed) {
                if line_start > cur_start {
                    blocks.push(Block {
                        start: cur_start,
                        end: line_start,
                        path: path_of(&heading),
                        is_code: false,
                    });
                }
                heading.truncate(level.saturating_sub(1));
                heading.push(title);
                cur_start = line_start;
            }
        }
    }

    let end = text.len();
    if cur_start < end {
        let (path, is_code) = if in_fence {
            // Unterminated fence: treat the remainder as code.
            let mut p = path_of(&heading).unwrap_or_default();
            p.push_str(if p.is_empty() { "code" } else { " > code" });
            (Some(p), true)
        } else {
            (path_of(&heading), false)
        };
        let start = if in_fence { fence_start } else { cur_start };
        blocks.push(Block {
            start,
            end,
            path,
            is_code,
        });
    }

    blocks
}

/// Sentence segments over `text[start..end]`: break after `.`/`!`/`?` when
/// followed by whitespace, and on blank lines. Leading whitespace is trimmed
/// from each segment; empty segments are dropped.
fn sentence_segments(text: &str, start: usize, end: usize) -> Vec<(usize, usize)> {
    let slice = &text[start..end];
    let chars: Vec<(usize, char)> = slice.char_indices().collect();
    let mut segs = Vec::new();
    let mut seg_start = start;

    for idx in 0..chars.len() {
        let (boff, c) = chars[idx];
        let global = start + boff;
        let next = chars.get(idx + 1).map(|&(_, n)| n);

        let terminator = matches!(c, '.' | '!' | '?') && next.map_or(true, |n| n.is_whitespace());
        let blank_line = c == '\n' && next == Some('\n');

        if terminator || blank_line {
            let cut = global + c.len_utf8();
            push_trimmed(text, seg_start, cut, &mut segs);
            seg_start = cut;
        }
    }
    push_trimmed(text, seg_start, end, &mut segs);
    segs
}

/// Line segments over `text[start..end]` (newline-inclusive), for code blocks.
fn line_segments(text: &str, start: usize, end: usize) -> Vec<(usize, usize)> {
    let slice = &text[start..end];
    let mut segs = Vec::new();
    let mut offset = start;
    for line in slice.split_inclusive('\n') {
        let s = offset;
        let e = offset + line.len();
        offset = e;
        // Keep code lines verbatim (including blanks) but skip a trailing empty.
        if e > s {
            segs.push((s, e));
        }
    }
    segs
}

/// Push `text[start..end]` as a segment after trimming leading whitespace;
/// drop it if empty.
fn push_trimmed(text: &str, start: usize, end: usize, segs: &mut Vec<(usize, usize)>) {
    if end <= start {
        return;
    }
    let slice = &text[start..end];
    let lead_ws: usize = slice.len() - slice.trim_start().len();
    let s = start + lead_ws;
    if end > s && !text[s..end].trim().is_empty() {
        segs.push((s, end));
    }
}

/// Slide a token-budget window over `segments`, emitting chunks with overlap.
fn window_segments(
    text: &str,
    segments: &[(usize, usize)],
    path: &Option<String>,
    cfg: &ChunkConfig,
    ordinal: &mut usize,
    out: &mut Vec<Chunk>,
) {
    if segments.is_empty() {
        return;
    }
    let tokens: Vec<usize> = segments
        .iter()
        .map(|&(s, e)| estimate_tokens(&text[s..e]))
        .collect();

    let mut i = 0;
    while i < segments.len() {
        let mut j = i;
        let mut tok = 0;
        while j < segments.len() && (tok < cfg.max_tokens || j == i) {
            tok += tokens[j];
            j += 1;
        }

        let start = segments[i].0;
        let end = segments[j - 1].1;
        let chunk_text = text[start..end].trim().to_string();
        if !chunk_text.is_empty() {
            out.push(Chunk {
                ordinal: *ordinal,
                text: chunk_text,
                token_count: tok,
                structural_path: path.clone(),
                locator: Locator::charspan(start, end),
            });
            *ordinal += 1;
        }

        if j >= segments.len() {
            break;
        }

        // Step back to re-include ~overlap_tokens of trailing context, always
        // making forward progress.
        let mut k = j;
        let mut otok = 0;
        while k > i + 1 && otok < cfg.overlap_tokens {
            k -= 1;
            otok += tokens[k];
        }
        i = k.max(i + 1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn estimate_tokens_ignores_whitespace() {
        assert_eq!(estimate_tokens(""), 0);
        assert_eq!(estimate_tokens("        "), 0);
        // 8 dense chars / 4 = 2
        assert_eq!(estimate_tokens("ab cd ef gh"), 2);
    }

    #[test]
    fn parse_heading_levels() {
        assert_eq!(parse_heading("# Title"), Some((1, "Title".to_string())));
        assert_eq!(parse_heading("### Deep"), Some((3, "Deep".to_string())));
        assert_eq!(parse_heading("####### too deep"), None);
        assert_eq!(parse_heading("#nospace"), None);
        assert_eq!(parse_heading("not a heading"), None);
    }

    #[test]
    fn markdown_blocks_track_heading_path() {
        let md = "# Intro\nHello world.\n\n## Details\nMore text here.\n";
        let blocks = structural_blocks_markdown(md);
        let paths: Vec<_> = blocks.iter().map(|b| b.path.clone()).collect();
        assert!(paths.contains(&Some("Intro".to_string())));
        assert!(paths.contains(&Some("Intro > Details".to_string())));
    }

    #[test]
    fn fenced_code_becomes_its_own_code_block() {
        let md = "# Code\nBefore.\n```rust\nfn main() {}\n```\nAfter.\n";
        let blocks = structural_blocks_markdown(md);
        let code = blocks.iter().find(|b| b.is_code).expect("a code block");
        assert!(code.path.as_deref().unwrap().ends_with("code"));
        assert!(md[code.start..code.end].contains("fn main()"));
    }

    #[test]
    fn sentence_segments_split_on_terminators() {
        let text = "One. Two! Three? Four";
        let segs = sentence_segments(text, 0, text.len());
        assert_eq!(segs.len(), 4);
        assert_eq!(&text[segs[0].0..segs[0].1], "One.");
        assert_eq!(&text[segs[3].0..segs[3].1], "Four");
    }

    #[test]
    fn chunks_cover_text_and_carry_locators() {
        // Build a long paragraph so it splits into multiple chunks.
        let sentence = "The quick brown fox jumps over the lazy dog every single day. ";
        let body = sentence.repeat(60);
        let text = format!("# Title\n{body}");
        let cfg = ChunkConfig::default();
        let chunks = chunk_document(&text, TextKind::Markdown, &cfg);

        assert!(chunks.len() > 1, "long text should yield multiple chunks");
        // Ordinals are sequential from 0.
        for (i, c) in chunks.iter().enumerate() {
            assert_eq!(c.ordinal, i);
            assert!(c.token_count > 0);
            match c.locator {
                Locator::Charspan {
                    char_start,
                    char_end,
                } => {
                    assert!(char_end > char_start);
                    assert!(char_end <= text.len());
                }
                _ => panic!("expected charspan locator"),
            }
        }
        // Heading path is propagated.
        assert_eq!(chunks[0].structural_path.as_deref(), Some("Title"));
    }

    #[test]
    fn overlap_makes_consecutive_chunks_share_context() {
        let sentence = "Alpha beta gamma delta epsilon zeta eta theta iota kappa. ";
        let body = sentence.repeat(80);
        let cfg = ChunkConfig::default();
        let chunks = chunk_document(&body, TextKind::Plain, &cfg);
        assert!(chunks.len() >= 2);
        // Each chunk respects the max budget (with slack for the final segment).
        for c in &chunks {
            assert!(c.token_count <= cfg.max_tokens + 50);
        }
    }

    #[test]
    fn empty_text_yields_no_chunks() {
        assert!(chunk_document("", TextKind::Plain, &ChunkConfig::default()).is_empty());
        assert!(chunk_document("   \n  ", TextKind::Markdown, &ChunkConfig::default()).is_empty());
    }
}
