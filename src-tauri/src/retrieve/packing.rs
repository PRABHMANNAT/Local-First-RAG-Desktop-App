//! Greedy token-budget packing.
//!
//! The answer prompt has a fixed context budget (default 6000 tokens). After
//! fusion and de-dup we have more candidate chunks than fit. We pack greedily
//! highest-score-first: walk the ranked list, admit a chunk if it fits in the
//! remaining budget, stop when the budget is exhausted. Greedy-by-score keeps
//! the most relevant context and is order-stable, which matters because the
//! prompt presents chunks in the packed order.

/// A candidate for packing: anything with a relevance order and a token cost.
/// Implemented for the retrieval pipeline's chunk type; generic here so the
/// packer stays unit-testable in isolation.
pub trait Packable {
    /// Token cost of including this item.
    fn token_count(&self) -> usize;
}

/// The default context budget in tokens, matching PLAN.md §6.
pub const DEFAULT_TOKEN_BUDGET: usize = 6000;

/// Greedily select items (assumed pre-sorted best-first) that fit within
/// `budget` tokens, returning the chosen items in their input order.
///
/// A single item larger than the entire budget is skipped, not truncated — the
/// caller's chunker is responsible for keeping chunks within a sane size, and
/// silently cutting context would corrupt citations. Once an item is admitted,
/// later smaller items may still be admitted if they fit ("best-fit tail"),
/// which uses the budget more fully than a hard stop at the first overflow.
pub fn pack<T: Packable>(items: Vec<T>, budget: usize) -> Vec<T> {
    let mut used = 0usize;
    let mut out = Vec::new();
    for item in items {
        let cost = item.token_count();
        if cost == 0 {
            continue;
        }
        if used + cost <= budget {
            used += cost;
            out.push(item);
        }
        // else: skip this item, keep scanning — a smaller later chunk may fit.
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Item {
        id: &'static str,
        tokens: usize,
    }
    impl Packable for Item {
        fn token_count(&self) -> usize {
            self.tokens
        }
    }

    fn items(spec: &[(&'static str, usize)]) -> Vec<Item> {
        spec.iter()
            .map(|&(id, tokens)| Item { id, tokens })
            .collect()
    }

    fn ids(v: &[Item]) -> Vec<&str> {
        v.iter().map(|i| i.id).collect()
    }

    #[test]
    fn packs_until_budget_is_full() {
        let picked = pack(items(&[("a", 3000), ("b", 3000), ("c", 3000)]), 6000);
        assert_eq!(ids(&picked), vec!["a", "b"]);
    }

    #[test]
    fn admits_smaller_tail_item_after_a_too_big_one() {
        // "b" (5000) doesn't fit in the 4000 left after "a"; "c" (1000) does.
        let picked = pack(items(&[("a", 2000), ("b", 5000), ("c", 1000)]), 6000);
        assert_eq!(ids(&picked), vec!["a", "c"]);
    }

    #[test]
    fn item_larger_than_budget_is_skipped_not_truncated() {
        let picked = pack(items(&[("huge", 10_000), ("ok", 500)]), 6000);
        assert_eq!(ids(&picked), vec!["ok"]);
    }

    #[test]
    fn preserves_input_order() {
        let picked = pack(items(&[("a", 100), ("b", 100), ("c", 100)]), 6000);
        assert_eq!(ids(&picked), vec!["a", "b", "c"]);
    }

    #[test]
    fn zero_cost_items_are_dropped() {
        let picked = pack(items(&[("empty", 0), ("real", 100)]), 6000);
        assert_eq!(ids(&picked), vec!["real"]);
    }

    #[test]
    fn empty_input_yields_empty() {
        let picked = pack(Vec::<Item>::new(), 6000);
        assert!(picked.is_empty());
    }
}
