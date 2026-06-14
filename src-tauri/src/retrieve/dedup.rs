//! Near-duplicate detection via MinHash over token shingles.
//!
//! After fusion the candidate set often holds near-identical chunks — the same
//! boilerplate header copied across files, a function and its doc-comment echo,
//! a paragraph quoted twice. Packing several of these wastes the token budget on
//! redundant context. MinHash gives a cheap, unbiased estimate of Jaccard
//! similarity between each chunk's shingle set, letting us drop near-dupes while
//! keeping the highest-ranked representative.
//!
//! We hash `w`-token shingles with `n` independent hash functions (built from a
//! single FNV-1a base hash mixed with per-function salts) and take the minimum
//! per function — the classic MinHash signature. The fraction of matching
//! signature slots between two chunks estimates their Jaccard similarity.

use crate::index::text::tokenize;

/// Number of hash functions in a signature. More slots → tighter Jaccard
/// estimate; 64 is plenty for the de-dup threshold we use.
const NUM_HASHES: usize = 64;
/// Shingle width in tokens. 3 captures local phrasing without being so long that
/// short chunks produce no shingles.
const SHINGLE_WIDTH: usize = 3;

/// Compute the MinHash signature of `text`. Returns `[u64; NUM_HASHES]`; an
/// empty or sub-shingle-width text yields an all-`u64::MAX` signature, which
/// compares as dissimilar to everything (estimated Jaccard 0).
fn signature(text: &str) -> [u64; NUM_HASHES] {
    let tokens = tokenize(text);
    let mut sig = [u64::MAX; NUM_HASHES];
    if tokens.len() < SHINGLE_WIDTH {
        // Fall back to single tokens so very short chunks still de-dupe exactly.
        for tok in &tokens {
            update_signature(&mut sig, fnv1a(tok.as_bytes()));
        }
        return sig;
    }
    for window in tokens.windows(SHINGLE_WIDTH) {
        let shingle = window.join(" ");
        update_signature(&mut sig, fnv1a(shingle.as_bytes()));
    }
    sig
}

/// Mix one shingle's base hash into every slot of the signature.
fn update_signature(sig: &mut [u64; NUM_HASHES], base: u64) {
    for (i, slot) in sig.iter_mut().enumerate() {
        // Distinct hash function per slot: xor a salt, then a multiplicative mix.
        let salted = (base ^ SALTS[i]).wrapping_mul(0x9e3779b97f4a7c15);
        if salted < *slot {
            *slot = salted;
        }
    }
}

/// Estimated Jaccard similarity = fraction of signature slots that agree.
fn estimated_jaccard(a: &[u64; NUM_HASHES], b: &[u64; NUM_HASHES]) -> f32 {
    let matches = a.iter().zip(b).filter(|(x, y)| x == y).count();
    matches as f32 / NUM_HASHES as f32
}

/// FNV-1a 64-bit hash of a byte slice.
fn fnv1a(bytes: &[u8]) -> u64 {
    let mut h: u64 = 0xcbf29ce484222325;
    for &b in bytes {
        h ^= b as u64;
        h = h.wrapping_mul(0x100000001b3);
    }
    h
}

/// Per-slot salts, derived deterministically so signatures are reproducible
/// across runs and processes.
const SALTS: [u64; NUM_HASHES] = build_salts();

const fn build_salts() -> [u64; NUM_HASHES] {
    let mut s = [0u64; NUM_HASHES];
    let mut i = 0;
    let mut state: u64 = 0x243f6a8885a308d3; // digits of pi, as a seed
    while i < NUM_HASHES {
        // SplitMix64 step — a good const-evaluable PRNG.
        state = state.wrapping_add(0x9e3779b97f4a7c15);
        let mut z = state;
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d049bb133111eb);
        s[i] = z ^ (z >> 31);
        i += 1;
    }
    s
}

/// Drop near-duplicate texts, keeping the first occurrence of each cluster.
///
/// Inputs are processed in order, so callers should pass the already-ranked
/// list — the highest-ranked representative of each duplicate cluster survives.
/// Two items are considered duplicates when their estimated Jaccard similarity
/// is `>= threshold` (e.g. `0.8`). Returns the indices of the kept items, in
/// input order.
pub fn dedupe_indices(texts: &[&str], threshold: f32) -> Vec<usize> {
    let mut kept: Vec<(usize, [u64; NUM_HASHES])> = Vec::new();
    let mut result = Vec::new();
    for (i, text) in texts.iter().enumerate() {
        let sig = signature(text);
        let is_dupe = kept
            .iter()
            .any(|(_, ksig)| estimated_jaccard(&sig, ksig) >= threshold);
        if !is_dupe {
            kept.push((i, sig));
            result.push(i);
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn salts_are_distinct() {
        // A collision would collapse two hash functions into one.
        let mut seen = std::collections::HashSet::new();
        for s in SALTS {
            assert!(seen.insert(s), "duplicate salt {s}");
        }
    }

    #[test]
    fn identical_texts_estimate_full_similarity() {
        let s = signature("the rocket engine ignites liquid oxygen to reach orbit");
        assert_eq!(estimated_jaccard(&s, &s), 1.0);
    }

    #[test]
    fn unrelated_texts_estimate_low_similarity() {
        let a = signature("knead the dough proof the yeast bake sourdough bread");
        let b = signature("the quarterly revenue report shows profit margins tax");
        assert!(estimated_jaccard(&a, &b) < 0.2);
    }

    #[test]
    fn keeps_first_of_a_near_duplicate_pair() {
        let texts = vec![
            "The rocket engine ignites liquid oxygen to reach orbit velocity.",
            // Same content, different casing/punctuation — the kind of dupe that
            // arises from boilerplate quoted across files. Tokenizes identically.
            "the ROCKET engine, ignites liquid oxygen — to reach orbit velocity!",
            "knead the dough proof the yeast then bake sourdough bread loaf",
        ];
        let kept = dedupe_indices(&texts, 0.8);
        // Index 1 (near-dupe of 0) is dropped; 0 and 2 survive.
        assert_eq!(kept, vec![0, 2]);
    }

    #[test]
    fn distinct_texts_all_survive() {
        let texts = vec![
            "alpha beta gamma delta",
            "epsilon zeta eta theta",
            "one two three four",
        ];
        let kept = dedupe_indices(&texts, 0.8);
        assert_eq!(kept, vec![0, 1, 2]);
    }

    #[test]
    fn empty_input_yields_empty() {
        assert!(dedupe_indices(&[], 0.8).is_empty());
    }
}
