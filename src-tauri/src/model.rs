//! Shared domain types used across ingest, retrieval, and the IPC surface.

use serde::{Deserialize, Serialize};

/// Where a chunk lives in its source document. Discriminated union mirrored by
/// the TypeScript `Locator` type; the source viewer drawer switches on `kind`
/// to render the cited span (PDF page box, code line range, transcript time).
///
/// Offsets are byte offsets into the document's UTF-8 text. For ASCII content
/// these equal character offsets; the frontend treats them as opaque positions.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum Locator {
    /// A span on a specific PDF page, optionally with a bounding box.
    Page {
        page: u32,
        char_start: usize,
        char_end: usize,
        #[serde(skip_serializing_if = "Option::is_none", default)]
        bbox: Option<[f32; 4]>,
    },
    /// A character span in flowing text (markdown, plain text, extracted URL).
    Charspan { char_start: usize, char_end: usize },
    /// A line range in a code file.
    Line { line_start: usize, line_end: usize },
    /// A time range (seconds) in an audio/video transcript.
    Time {
        start_seconds: f64,
        end_seconds: f64,
    },
}

impl Locator {
    /// Convenience constructor for the common flowing-text case.
    pub fn charspan(start: usize, end: usize) -> Self {
        Locator::Charspan {
            char_start: start,
            char_end: end,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn charspan_round_trips_through_json() {
        let loc = Locator::charspan(10, 42);
        let json = serde_json::to_string(&loc).unwrap();
        assert_eq!(json, r#"{"kind":"charspan","char_start":10,"char_end":42}"#);
        let back: Locator = serde_json::from_str(&json).unwrap();
        assert_eq!(back, loc);
    }

    #[test]
    fn page_omits_bbox_when_absent() {
        let loc = Locator::Page {
            page: 1,
            char_start: 0,
            char_end: 5,
            bbox: None,
        };
        let json = serde_json::to_string(&loc).unwrap();
        assert!(
            !json.contains("bbox"),
            "absent bbox should be omitted: {json}"
        );
    }

    #[test]
    fn time_locator_tag() {
        let loc = Locator::Time {
            start_seconds: 1.5,
            end_seconds: 9.0,
        };
        let v = serde_json::to_value(&loc).unwrap();
        assert_eq!(v["kind"], "time");
        assert_eq!(v["start_seconds"], 1.5);
    }
}
