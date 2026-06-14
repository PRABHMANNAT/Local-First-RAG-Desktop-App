//! Reciprocal Rank Fusion (RRF).
//!
//! Vector search and BM25 produce scores on incomparable scales (cosine vs.
//! Okapi). RRF sidesteps the calibration problem entirely by fusing on *rank*
//! rather than score: a document's fused score is the sum, over every ranked
//! list it appears in, of `1 / (k + rank)`. The constant `k` (default 60, from
//! Cormack et al. 2009) damps the influence of top ranks so a single list can't
//! dominate.
//!
//! ```text
//! RRF(d) = Σ_lists  1 / (k + rank_list(d))      // rank is 1-based
//! ```

use crate::index::ScoredChunk;

/// The canonical RRF damping constant from the original paper.
pub const DEFAULT_RRF_K: f32 = 60.0;

/// Fuse several ranked candidate lists into one, best first.
///
/// Each input list is assumed already sorted best-first; only the *position*
/// within each list matters, not the raw score. The returned [`ScoredChunk`]s
/// carry the fused RRF score (not comparable to the inputs' scores). Ties break
/// by chunk id for determinism.
pub fn reciprocal_rank_fusion(lists: &[Vec<ScoredChunk>], k: f32) -> Vec<ScoredChunk> {
    use std::collections::HashMap;

    let mut fused: HashMap<String, f32> = HashMap::new();
    for list in lists {
        for (rank, hit) in list.iter().enumerate() {
            let contribution = 1.0 / (k + (rank as f32 + 1.0));
            *fused.entry(hit.chunk_id.clone()).or_insert(0.0) += contribution;
        }
    }

    let mut out: Vec<ScoredChunk> = fused
        .into_iter()
        .map(|(chunk_id, score)| ScoredChunk { chunk_id, score })
        .collect();

    out.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.chunk_id.cmp(&b.chunk_id))
    });
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hit(id: &str) -> ScoredChunk {
        ScoredChunk {
            chunk_id: id.into(),
            // Raw score is intentionally arbitrary — RRF must ignore it.
            score: 999.0,
        }
    }

    #[test]
    fn empty_input_fuses_to_empty() {
        assert!(reciprocal_rank_fusion(&[], DEFAULT_RRF_K).is_empty());
    }

    #[test]
    fn single_list_preserves_its_order() {
        let list = vec![hit("a"), hit("b"), hit("c")];
        let fused = reciprocal_rank_fusion(&[list], DEFAULT_RRF_K);
        let ids: Vec<_> = fused.iter().map(|h| h.chunk_id.as_str()).collect();
        assert_eq!(ids, vec!["a", "b", "c"]);
    }

    #[test]
    fn agreement_across_lists_outranks_a_single_top_hit() {
        // "b" is rank-1 in one list only. "a" is rank-2 in both lists; its two
        // contributions should sum higher than b's single rank-1 contribution.
        let vec_list = vec![hit("b"), hit("a")];
        let bm25_list = vec![hit("c"), hit("a")];
        let fused = reciprocal_rank_fusion(&[vec_list, bm25_list], DEFAULT_RRF_K);
        assert_eq!(fused[0].chunk_id, "a");
    }

    #[test]
    fn raw_scores_are_ignored_only_rank_matters() {
        let strong = ScoredChunk {
            chunk_id: "x".into(),
            score: 0.0001,
        };
        let weak = ScoredChunk {
            chunk_id: "y".into(),
            score: 100000.0,
        };
        // x is ranked first despite a tiny raw score.
        let fused = reciprocal_rank_fusion(&[vec![strong, weak]], DEFAULT_RRF_K);
        assert_eq!(fused[0].chunk_id, "x");
    }

    #[test]
    fn ties_break_by_chunk_id() {
        // Same rank in symmetric lists → identical fused score → id order.
        let l1 = vec![hit("b"), hit("a")];
        let l2 = vec![hit("a"), hit("b")];
        let fused = reciprocal_rank_fusion(&[l1, l2], DEFAULT_RRF_K);
        assert_eq!(fused[0].chunk_id, "a");
        assert_eq!(fused[1].chunk_id, "b");
    }
}
