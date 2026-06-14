//! Repo source: shallow-clone a public Git URL into the workspace cache, then
//! reuse the folder walk to ingest its text files. Cloning shells out to the
//! `git` binary (statically bundled per PLAN §14); the pure URL/parse helpers
//! are unit-tested here, while the clone itself is exercised in manual QA.

use std::path::{Path, PathBuf};
use std::process::Command;

use crate::error::{AppError, AppResult};

/// A parsed, validated Git remote. We accept `https://`, `http://`, `git://`,
/// and `git@host:org/repo(.git)` SSH-style forms.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GitRemote {
    pub url: String,
    /// `org/repo`, used to name the cache directory and the source title.
    pub slug: String,
}

/// Parse and validate a Git URL, deriving its `org/repo` slug. Rejects obvious
/// non-URLs early so the UI can surface a clear error before any network I/O.
pub fn parse_git_url(input: &str) -> AppResult<GitRemote> {
    let url = input.trim();
    if url.is_empty() {
        return Err(AppError::InvalidArgument("empty git url".into()));
    }

    // Normalize the path portion we slugify from.
    let path_part = if let Some(rest) = url.strip_prefix("git@") {
        // git@github.com:org/repo.git  ->  org/repo.git
        rest.split_once(':').map(|(_, p)| p).unwrap_or("")
    } else if let Some(idx) = url.find("://") {
        // scheme://host/org/repo(.git)  ->  org/repo(.git), drop host
        let after = &url[idx + 3..];
        after.split_once('/').map(|(_, p)| p).unwrap_or("")
    } else {
        return Err(AppError::InvalidArgument(format!(
            "unrecognized git url: {url}"
        )));
    };

    let slug = path_part.trim_matches('/').trim_end_matches(".git");
    if slug.is_empty() || !slug.contains('/') {
        return Err(AppError::InvalidArgument(format!(
            "could not derive org/repo from: {url}"
        )));
    }

    Ok(GitRemote {
        url: url.to_string(),
        slug: slug.to_string(),
    })
}

/// Derive a filesystem-safe cache directory name for a remote, under
/// `cache_root`. Slashes and other separators are flattened so `org/repo`
/// becomes `org__repo`.
pub fn cache_dir(cache_root: &Path, remote: &GitRemote) -> PathBuf {
    // `/` becomes `__` so `org/repo` is recoverable and can't collide with an
    // `org_repo` slug; any other non-portable char becomes a single `_`.
    let safe: String = remote
        .slug
        .chars()
        .flat_map(|c| {
            if c == '/' {
                vec!['_', '_']
            } else if c.is_alphanumeric() || c == '-' || c == '_' {
                vec![c]
            } else {
                vec!['_']
            }
        })
        .collect();
    cache_root.join(safe)
}

/// Shallow-clone (`--depth 1`) `remote` into `dest`, or fetch+reset if it
/// already exists (the user "Sync" path). Returns the checkout directory.
///
/// Network egress happens only here, on an explicit add/sync action (PLAN §9).
pub fn clone_or_sync(remote: &GitRemote, dest: &Path) -> AppResult<PathBuf> {
    let status = if dest.join(".git").exists() {
        Command::new("git")
            .arg("-C")
            .arg(dest)
            .args(["fetch", "--depth", "1", "origin"])
            .status()
    } else {
        Command::new("git")
            .args(["clone", "--depth", "1", &remote.url])
            .arg(dest)
            .status()
    }
    .map_err(|e| AppError::Other(format!("git not available: {e}")))?;

    if !status.success() {
        return Err(AppError::Other(format!(
            "git clone/fetch failed for {} (exit {:?})",
            remote.url,
            status.code()
        )));
    }
    Ok(dest.to_path_buf())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_https_url() {
        let r =
            parse_git_url("https://github.com/PRABHMANNAT/Local-First-RAG-Desktop-App").unwrap();
        assert_eq!(r.slug, "PRABHMANNAT/Local-First-RAG-Desktop-App");
    }

    #[test]
    fn parses_https_url_with_dot_git_suffix() {
        let r = parse_git_url("https://github.com/rust-lang/cargo.git").unwrap();
        assert_eq!(r.slug, "rust-lang/cargo");
    }

    #[test]
    fn parses_ssh_style_url() {
        let r = parse_git_url("git@github.com:tokio-rs/tokio.git").unwrap();
        assert_eq!(r.slug, "tokio-rs/tokio");
    }

    #[test]
    fn rejects_garbage_and_empty() {
        assert!(parse_git_url("").is_err());
        assert!(parse_git_url("not a url").is_err());
        assert!(parse_git_url("https://github.com/").is_err());
    }

    #[test]
    fn cache_dir_flattens_slug() {
        let r = parse_git_url("https://github.com/rust-lang/cargo").unwrap();
        let dir = cache_dir(Path::new("/cache"), &r);
        // Compare the leaf only, so the OS path separator doesn't matter.
        assert_eq!(
            dir.file_name().unwrap().to_str().unwrap(),
            "rust-lang__cargo"
        );
        assert_eq!(dir.parent().unwrap(), Path::new("/cache"));
    }
}
