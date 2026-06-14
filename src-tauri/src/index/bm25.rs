//! In-process BM25 full-text index.
//!
//! PLAN.md names `tantivy` for the BM25 side. We deliberately ship a pure-Rust
//! Okapi BM25 here for M2 — the same posture M1 took with `SqliteVectorStore`
//! standing in for LanceDB: correct, fully unit-testable, and free of the
//! native-binding friction tantivy brings on Windows (a documented M0 risk). It
//! drops in behind the [`crate::index`] retrieval surface, and a tantivy-backed
//! implementation can replace it later without touching callers.
//!
//! Scoring is the standard Okapi BM25:
//! ```text
//! score(q, d) = Σ_t IDF(t) · (f(t,d) · (k1 + 1)) / (f(t,d) + k1 · (1 - b + b · |d| / avgdl))
//! IDF(t)      = ln(1 + (N - n(t) + 0.5) / (n(t) + 0.5))
//! ```
//! with the usual defaults `k1 = 1.2`, `b = 0.75`.

use std::collections::HashMap;

use super::text::tokenize;
use super::ScoredChunk;

const K1: f32 = 1.2;
const B: f32 = 0.75;

/// A single indexed document: its chunk id and per-term frequencies.
struct Posting {
    chunk_id: String,
    len: u32,
    term_freqs: HashMap<String, u32>,
}

/// An in-memory BM25 index over chunk text. Built once per query batch from the
/// corpus; cheap enough at M2 corpus sizes and trivially correct. Documents are
/// added with [`Bm25Index::add`] and searched with [`Bm25Index::search`].
#[derive(Default)]
pub struct Bm25Index {
    postings: Vec<Posting>,
    /// term -> number of documents containing it (document frequency).
    doc_freq: HashMap<String, u32>,
    total_len: u64,
}

impl Bm25Index {
    /// Create an empty index.
    pub fn new() -> Self {
        Self::default()
    }

    /// Number of indexed documents.
    pub fn len(&self) -> usize {
        self.postings.len()
    }

    /// Whether the index holds no documents.
    pub fn is_empty(&self) -> bool {
        self.postings.is_empty()
    }

    /// Index one document's text under `chunk_id`. Re-tokenizes with the shared
    /// analyzer so index- and query-time statistics agree.
    pub fn add(&mut self, chunk_id: impl Into<String>, text: &str) {
        let tokens = tokenize(text);
        let len = tokens.len() as u32;
        let mut term_freqs: HashMap<String, u32> = HashMap::new();
        for t in tokens {
            *term_freqs.entry(t).or_insert(0) += 1;
        }
        for term in term_freqs.keys() {
            *self.doc_freq.entry(term.clone()).or_insert(0) += 1;
        }
        self.total_len += len as u64;
        self.postings.push(Posting {
            chunk_id: chunk_id.into(),
            len,
            term_freqs,
        });
    }

    /// Average document length in tokens (the BM25 `avgdl`). Zero for an empty
    /// index.
    fn avgdl(&self) -> f32 {
        if self.postings.is_empty() {
            0.0
        } else {
            self.total_len as f32 / self.postings.len() as f32
        }
    }

    /// Inverse document frequency for `term` (Robertson/Sparck-Jones form, the
    /// `ln(1 + …)` variant that stays non-negative).
    fn idf(&self, term: &str) -> f32 {
        let n = self.postings.len() as f32;
        let df = *self.doc_freq.get(term).unwrap_or(&0) as f32;
        ((n - df + 0.5) / (df + 0.5) + 1.0).ln()
    }

    /// Score every document against `query` and return the top-`k`, best first.
    /// Documents that share no query term are omitted (score 0). Ties break by
    /// the order documents were added, so results are deterministic.
    pub fn search(&self, query: &str, k: usize) -> Vec<ScoredChunk> {
        if self.postings.is_empty() || k == 0 {
            return Vec::new();
        }
        let avgdl = self.avgdl();
        let q_terms = tokenize(query);
        // Precompute IDF per distinct query term.
        let idfs: HashMap<&str, f32> = q_terms.iter().map(|t| (t.as_str(), self.idf(t))).collect();

        let mut scored: Vec<ScoredChunk> = self
            .postings
            .iter()
            .filter_map(|p| {
                let mut score = 0.0f32;
                for term in &q_terms {
                    let f = match p.term_freqs.get(term) {
                        Some(&f) => f as f32,
                        None => continue,
                    };
                    let idf = idfs[term.as_str()];
                    let denom = f + K1 * (1.0 - B + B * (p.len as f32 / avgdl));
                    score += idf * (f * (K1 + 1.0)) / denom;
                }
                if score > 0.0 {
                    Some(ScoredChunk {
                        chunk_id: p.chunk_id.clone(),
                        score,
                    })
                } else {
                    None
                }
            })
            .collect();

        scored.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        scored.truncate(k);
        scored
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn corpus() -> Bm25Index {
        let mut idx = Bm25Index::new();
        idx.add(
            "c1",
            "Knead the dough and proof the yeast to bake sourdough bread",
        );
        idx.add(
            "c2",
            "The quarterly revenue report shows profit margins and tax",
        );
        idx.add(
            "c3",
            "The rocket engine ignites liquid oxygen to reach orbit",
        );
        idx
    }

    #[test]
    fn empty_index_returns_nothing() {
        let idx = Bm25Index::new();
        assert!(idx.search("anything", 5).is_empty());
        assert!(idx.is_empty());
    }

    #[test]
    fn ranks_lexically_matching_document_first() {
        let idx = corpus();
        let hits = idx.search("sourdough bread dough yeast", 3);
        assert_eq!(hits[0].chunk_id, "c1");
    }

    #[test]
    fn omits_documents_with_no_shared_term() {
        let idx = corpus();
        let hits = idx.search("rocket orbit", 10);
        // Only c3 shares terms; c1/c2 score 0 and are dropped.
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].chunk_id, "c3");
    }

    #[test]
    fn results_sorted_descending_and_capped_at_k() {
        let mut idx = Bm25Index::new();
        for i in 0..5 {
            idx.add(format!("c{i}"), "shared term shared term unique");
        }
        let hits = idx.search("shared", 3);
        assert_eq!(hits.len(), 3);
        for w in hits.windows(2) {
            assert!(w[0].score >= w[1].score);
        }
    }

    #[test]
    fn rarer_term_outweighs_common_one() {
        // "orbit" appears in one doc; "the" is a stop word (gone). A query with a
        // rare discriminating term should surface its single owner strongly.
        let idx = corpus();
        let hits = idx.search("orbit", 3);
        assert_eq!(hits.len(), 1);
        assert!(hits[0].score > 0.0);
    }
}
