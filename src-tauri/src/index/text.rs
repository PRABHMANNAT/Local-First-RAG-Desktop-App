//! Text analysis for the BM25 index. A deliberately small, dependency-free
//! analyzer: lowercase, split on non-alphanumerics, drop a tiny English
//! stop-word set, and keep tokens of length ≥ 2. The same analyzer must run at
//! index time and query time so term statistics line up — callers should never
//! tokenize by hand.

/// A compact English stop-word set. Kept intentionally short: aggressive
/// stop-word removal hurts code and technical prose, where words like "in" or
/// "for" can be meaningful (e.g. `for` loops). These are the highest-frequency
/// function words that carry no retrieval signal.
const STOP_WORDS: &[&str] = &[
    "a", "an", "and", "are", "as", "at", "be", "by", "for", "from", "has", "he", "in", "is", "it",
    "its", "of", "on", "that", "the", "to", "was", "were", "will", "with",
];

/// Tokenize `text` into lowercase terms suitable for BM25 indexing and querying.
///
/// Splitting is on any character that is not a Unicode alphanumeric, so
/// `snake_case` and `kebab-case` identifiers break into their parts while
/// numbers survive. Stop words and single characters are dropped.
pub fn tokenize(text: &str) -> Vec<String> {
    text.split(|c: char| !c.is_alphanumeric())
        .filter(|s| !s.is_empty())
        .map(|s| s.to_lowercase())
        .filter(|s| s.chars().count() >= 2 && !is_stop_word(s))
        .collect()
}

/// Whether `term` (already lowercased) is in the stop-word set.
fn is_stop_word(term: &str) -> bool {
    STOP_WORDS.binary_search(&term).is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stop_words_are_sorted_for_binary_search() {
        let mut sorted = STOP_WORDS.to_vec();
        sorted.sort_unstable();
        assert_eq!(STOP_WORDS, sorted.as_slice(), "STOP_WORDS must stay sorted");
    }

    #[test]
    fn lowercases_and_splits_on_punctuation() {
        assert_eq!(tokenize("Hello, World!"), vec!["hello", "world"]);
    }

    #[test]
    fn splits_snake_and_kebab_identifiers() {
        assert_eq!(
            tokenize("parse_pdf_page render-box"),
            vec!["parse", "pdf", "page", "render", "box"]
        );
    }

    #[test]
    fn drops_stop_words_and_single_chars() {
        // "the" and "a" are stop words; "x" is a single char.
        assert_eq!(tokenize("the quick x brown"), vec!["quick", "brown"]);
    }

    #[test]
    fn keeps_numbers_and_alphanumerics() {
        assert_eq!(tokenize("v2 utf8 3d"), vec!["v2", "utf8", "3d"]);
    }

    #[test]
    fn empty_text_yields_no_tokens() {
        assert!(tokenize("   ,.;  ").is_empty());
    }
}
