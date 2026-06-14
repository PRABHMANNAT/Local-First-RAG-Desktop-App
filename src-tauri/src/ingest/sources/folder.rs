//! Folder source: a gitignore-aware recursive walk that yields the text files
//! we know how to chunk, plus content hashing for idempotent re-ingest.

use std::path::{Path, PathBuf};

use crate::ingest::chunker::TextKind;

/// Which file types to ingest, by lowercase extension. Defaults cover markdown,
/// plain text, and common code files. PDFs/DOCX are handled by dedicated parsers
/// (M2+), not this text path.
#[derive(Debug, Clone)]
pub struct IncludeConfig {
    pub extensions: Vec<String>,
}

impl Default for IncludeConfig {
    fn default() -> Self {
        let exts = [
            // prose
            "md", "markdown", "mdx", "txt", "rst", "org", // code
            "rs", "ts", "tsx", "js", "jsx", "py", "go", "java", "kt", "c", "h", "cpp", "hpp", "cc",
            "cs", "rb", "php", "swift", "scala", "sh", "bash", "toml", "yaml", "yml", "json",
            "css", "scss", "html", "sql",
        ];
        Self {
            extensions: exts.iter().map(|s| s.to_string()).collect(),
        }
    }
}

impl IncludeConfig {
    fn allows(&self, ext: &str) -> bool {
        let lower = ext.to_ascii_lowercase();
        self.extensions.iter().any(|e| e == &lower)
    }
}

/// A file discovered by the walk, with its resolved chunking kind.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WalkedFile {
    pub path: PathBuf,
    pub text_kind: TextKind,
}

/// Map an extension to how its text should be chunked. Returns `None` for types
/// this text path doesn't handle.
pub fn text_kind_for_ext(ext: &str) -> Option<TextKind> {
    match ext.to_ascii_lowercase().as_str() {
        "md" | "markdown" | "mdx" => Some(TextKind::Markdown),
        "txt" | "rst" | "org" => Some(TextKind::Plain),
        "rs" | "ts" | "tsx" | "js" | "jsx" | "py" | "go" | "java" | "kt" | "c" | "h" | "cpp"
        | "hpp" | "cc" | "cs" | "rb" | "php" | "swift" | "scala" | "sh" | "bash" | "toml"
        | "yaml" | "yml" | "json" | "css" | "scss" | "html" | "sql" => Some(TextKind::Code),
        _ => None,
    }
}

/// Recursively walk `root`, respecting `.gitignore`/`.ignore` and skipping
/// hidden files, returning the text files we can ingest. Results are sorted by
/// path for deterministic ordering.
pub fn walk(root: &Path, cfg: &IncludeConfig) -> Vec<WalkedFile> {
    let mut out = Vec::new();
    // `require_git(false)` makes `.gitignore` apply to plain folders too, not
    // only inside a git repo — folders dropped into Mnemos often aren't repos.
    // `.mnemosignore` is layered on top with identical gitignore semantics
    // (PLAN §5), so user excludes compose with the repo's own ignore rules.
    let walker = ignore::WalkBuilder::new(root)
        .hidden(true)
        .require_git(false)
        .add_custom_ignore_filename(super::ignore_rules::MNEMOSIGNORE)
        .build();
    for entry in walker.flatten() {
        if !entry.file_type().is_some_and(|t| t.is_file()) {
            continue;
        }
        let path = entry.into_path();
        let Some(ext) = path.extension().and_then(|e| e.to_str()) else {
            continue;
        };
        if !cfg.allows(ext) {
            continue;
        }
        if let Some(text_kind) = text_kind_for_ext(ext) {
            out.push(WalkedFile { path, text_kind });
        }
    }
    out.sort_by(|a, b| a.path.cmp(&b.path));
    out
}

/// Blake3 content hash, hex-encoded. Drives idempotent re-ingest: unchanged
/// content hashes to the same value and is skipped.
pub fn content_hash(bytes: &[u8]) -> String {
    blake3::hash(bytes).to_hex().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn text_kind_mapping() {
        assert_eq!(text_kind_for_ext("MD"), Some(TextKind::Markdown));
        assert_eq!(text_kind_for_ext("rs"), Some(TextKind::Code));
        assert_eq!(text_kind_for_ext("txt"), Some(TextKind::Plain));
        assert_eq!(text_kind_for_ext("png"), None);
    }

    #[test]
    fn content_hash_is_stable_and_distinct() {
        assert_eq!(content_hash(b"hello"), content_hash(b"hello"));
        assert_ne!(content_hash(b"hello"), content_hash(b"world"));
    }

    #[test]
    fn walk_finds_text_files_and_skips_unknown() {
        let dir = tempdir().unwrap();
        let root = dir.path();
        fs::write(root.join("a.md"), "# A").unwrap();
        fs::write(root.join("b.rs"), "fn main() {}").unwrap();
        fs::write(root.join("c.png"), [0u8, 1, 2]).unwrap();

        let files = walk(root, &IncludeConfig::default());
        let names: Vec<_> = files
            .iter()
            .map(|f| f.path.file_name().unwrap().to_string_lossy().to_string())
            .collect();
        assert!(names.contains(&"a.md".to_string()));
        assert!(names.contains(&"b.rs".to_string()));
        assert!(!names.contains(&"c.png".to_string()));
    }

    #[test]
    fn walk_respects_mnemosignore() {
        let dir = tempdir().unwrap();
        let root = dir.path();
        fs::write(root.join(".mnemosignore"), "vendor/\n*.gen.ts\n").unwrap();
        fs::write(root.join("keep.ts"), "export const a = 1;").unwrap();
        fs::write(root.join("schema.gen.ts"), "// generated").unwrap();
        fs::create_dir(root.join("vendor")).unwrap();
        fs::write(root.join("vendor").join("lib.ts"), "// vendored").unwrap();

        let files = walk(root, &IncludeConfig::default());
        let names: Vec<_> = files
            .iter()
            .map(|f| f.path.file_name().unwrap().to_string_lossy().to_string())
            .collect();
        assert!(names.contains(&"keep.ts".to_string()));
        assert!(!names.contains(&"schema.gen.ts".to_string()), "{names:?}");
        assert!(!names.contains(&"lib.ts".to_string()), "{names:?}");
    }

    #[test]
    fn walk_respects_gitignore() {
        let dir = tempdir().unwrap();
        let root = dir.path();
        fs::write(root.join(".gitignore"), "ignored.md\n").unwrap();
        fs::write(root.join("kept.md"), "# kept").unwrap();
        fs::write(root.join("ignored.md"), "# ignored").unwrap();

        let files = walk(root, &IncludeConfig::default());
        let names: Vec<_> = files
            .iter()
            .map(|f| f.path.file_name().unwrap().to_string_lossy().to_string())
            .collect();
        assert!(names.contains(&"kept.md".to_string()));
        assert!(
            !names.contains(&"ignored.md".to_string()),
            "gitignored file should be skipped: {names:?}"
        );
    }
}
