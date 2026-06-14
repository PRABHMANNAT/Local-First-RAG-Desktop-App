//! URL source: fetch a web page and extract its readable text for chunking.
//!
//! The fetch (network egress, only on explicit add/sync per PLAN §9) uses the
//! `reqwest` client already in the tree. Readability is a small, dependency-free
//! extraction: drop `<script>`/`<style>`/`<head>` content, strip remaining tags,
//! collapse whitespace, and pull the `<title>`. It deliberately avoids a heavy
//! DOM/readability crate for M2/M3 — good enough for cited text spans, and the
//! pure extractor is fully unit-testable without a network.

use crate::error::{AppError, AppResult};

/// A fetched, extracted web document.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExtractedPage {
    pub title: Option<String>,
    pub text: String,
}

/// Extract a page title and readable body text from raw HTML. Content inside
/// `<script>`, `<style>`, `<head>`, and HTML comments is removed; remaining tags
/// are stripped and whitespace collapsed to single spaces.
pub fn extract_readable(html: &str) -> ExtractedPage {
    let title = extract_title(html);
    let body = strip_region(html, "<head", "</head>");
    let body = strip_region(&body, "<script", "</script>");
    let body = strip_region(&body, "<style", "</style>");
    let body = strip_comments(&body);
    let text = collapse_whitespace(&strip_tags(&body));
    ExtractedPage { title, text }
}

/// Pull the text inside the first `<title>…</title>`, trimmed.
fn extract_title(html: &str) -> Option<String> {
    let lower = html.to_lowercase();
    let open = lower.find("<title")?;
    let gt = lower[open..].find('>')? + open + 1;
    let close = lower[gt..].find("</title>")? + gt;
    let title = html[gt..close].trim();
    if title.is_empty() {
        None
    } else {
        Some(collapse_whitespace(title))
    }
}

/// Remove every region from a case-insensitive `open` tag-prefix to its matching
/// `close`, inclusive. Used to drop non-content elements.
fn strip_region(html: &str, open: &str, close: &str) -> String {
    let mut out = String::with_capacity(html.len());
    let lower = html.to_lowercase();
    let mut i = 0;
    while i < html.len() {
        if lower[i..].starts_with(open) {
            if let Some(end_rel) = lower[i..].find(close) {
                i += end_rel + close.len();
                continue;
            } else {
                break; // unterminated region: drop the rest
            }
        }
        let ch = html[i..].chars().next().unwrap();
        out.push(ch);
        i += ch.len_utf8();
    }
    out
}

/// Remove `<!-- … -->` comments.
fn strip_comments(html: &str) -> String {
    let mut out = String::with_capacity(html.len());
    let mut rest = html;
    while let Some(start) = rest.find("<!--") {
        out.push_str(&rest[..start]);
        if let Some(end) = rest[start..].find("-->") {
            rest = &rest[start + end + 3..];
        } else {
            return out; // unterminated comment: drop the rest
        }
    }
    out.push_str(rest);
    out
}

/// Strip all `<…>` tags, replacing each with a space so adjacent words don't
/// fuse together.
fn strip_tags(html: &str) -> String {
    let mut out = String::with_capacity(html.len());
    let mut in_tag = false;
    for ch in html.chars() {
        match ch {
            '<' => in_tag = true,
            '>' => {
                in_tag = false;
                out.push(' ');
            }
            _ if !in_tag => out.push(ch),
            _ => {}
        }
    }
    out
}

/// Collapse all runs of ASCII whitespace to single spaces and trim.
fn collapse_whitespace(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Fetch `url` and return its extracted readable content. The only network call
/// in this module; callers gate it behind explicit user action.
pub async fn fetch_and_extract(url: &str) -> AppResult<ExtractedPage> {
    let resp = reqwest::get(url)
        .await
        .map_err(|e| AppError::Other(format!("fetch failed: {e}")))?;
    let html = resp
        .text()
        .await
        .map_err(|e| AppError::Other(format!("read body failed: {e}")))?;
    Ok(extract_readable(&html))
}

#[cfg(test)]
mod tests {
    use super::*;

    const PAGE: &str = r#"
        <html>
          <head><title>  Hello   World  </title><style>.x{color:red}</style></head>
          <body>
            <!-- nav comment -->
            <script>var a = 1;</script>
            <h1>Heading</h1>
            <p>First paragraph of <b>real</b> content.</p>
          </body>
        </html>"#;

    #[test]
    fn extracts_title() {
        assert_eq!(extract_readable(PAGE).title.as_deref(), Some("Hello World"));
    }

    #[test]
    fn drops_script_style_head_and_comments() {
        let text = extract_readable(PAGE).text;
        assert!(!text.contains("color:red"), "style leaked: {text}");
        assert!(!text.contains("var a"), "script leaked: {text}");
        assert!(!text.contains("nav comment"), "comment leaked: {text}");
        assert!(
            !text.contains("Hello World"),
            "head title leaked into body: {text}"
        );
    }

    #[test]
    fn keeps_body_text_without_tags() {
        let text = extract_readable(PAGE).text;
        assert!(text.contains("Heading"));
        assert!(text.contains("First paragraph of real content."));
        assert!(!text.contains('<'));
    }

    #[test]
    fn missing_title_is_none() {
        let page = extract_readable("<html><body><p>no title here</p></body></html>");
        assert_eq!(page.title, None);
        assert_eq!(page.text, "no title here");
    }
}
