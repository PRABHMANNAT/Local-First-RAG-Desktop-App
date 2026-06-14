//! `.mnemosignore` support.
//!
//! PLAN.md §5 says `.mnemosignore` *augments* `.gitignore` for folder and repo
//! sources. The folder walk already honors `.gitignore`/`.ignore` via the
//! `ignore` crate; we register `.mnemosignore` as an additional custom ignore
//! filename so its patterns compose with gitignore using the exact same,
//! battle-tested gitignore semantics (negation, `**`, anchoring, directory-only
//! `dir/`). This module also exposes a standalone [`MnemosIgnore`] matcher for
//! callers that need to test a path against patterns without walking a tree
//! (e.g. the repo source filtering a clone listing).

use ignore::gitignore::{Gitignore, GitignoreBuilder};
use std::path::Path;

/// The custom ignore filename Mnemos layers on top of `.gitignore`.
pub const MNEMOSIGNORE: &str = ".mnemosignore";

/// A compiled set of `.mnemosignore` patterns rooted at a directory. Thin
/// wrapper over the `ignore` crate's gitignore matcher so patterns behave
/// exactly like gitignore.
pub struct MnemosIgnore {
    inner: Gitignore,
}

impl MnemosIgnore {
    /// Build a matcher from explicit pattern lines rooted at `root`. Lines use
    /// gitignore syntax; blank lines and `#` comments are ignored.
    pub fn from_lines(root: &Path, lines: &[&str]) -> Self {
        let mut builder = GitignoreBuilder::new(root);
        for line in lines {
            // add_line never fails for our inputs, but ignore errors defensively.
            let _ = builder.add_line(None, line);
        }
        let inner = builder.build().unwrap_or_else(|_| Gitignore::empty());
        Self { inner }
    }

    /// Whether `path` (a file) is ignored by these patterns.
    pub fn is_ignored(&self, path: &Path) -> bool {
        self.inner.matched(path, /* is_dir */ false).is_ignore()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn root() -> PathBuf {
        PathBuf::from("/repo")
    }

    #[test]
    fn matches_simple_glob() {
        let ig = MnemosIgnore::from_lines(&root(), &["*.log", "target/"]);
        assert!(ig.is_ignored(&root().join("debug.log")));
        assert!(!ig.is_ignored(&root().join("main.rs")));
    }

    #[test]
    fn supports_double_star_and_negation() {
        let ig = MnemosIgnore::from_lines(&root(), &["**/dist/**", "!keep/dist/important.js"]);
        assert!(ig.is_ignored(&root().join("a/b/dist/bundle.js")));
        assert!(!ig.is_ignored(&root().join("keep/dist/important.js")));
    }

    #[test]
    fn comments_and_blank_lines_are_inert() {
        let ig = MnemosIgnore::from_lines(&root(), &["# a comment", "", "secret.txt"]);
        assert!(ig.is_ignored(&root().join("secret.txt")));
        assert!(!ig.is_ignored(&root().join("a")));
    }

    #[test]
    fn empty_ruleset_ignores_nothing() {
        let ig = MnemosIgnore::from_lines(&root(), &[]);
        assert!(!ig.is_ignored(&root().join("anything.rs")));
    }
}
