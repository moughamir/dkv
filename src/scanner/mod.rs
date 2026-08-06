use std::path::Path;

use walkdir::WalkDir;

use crate::error::Result;

mod ignore;

pub use ignore::{ExclusionPattern, is_excluded, load_ignore_files, matches_exclusion};

/// Filesystem scanner that supports configurable exclusions and
/// `.gitignore` / `.dkvignore` patterns.
pub struct Scanner {
    exclusions: Vec<ExclusionPattern>,
    config_exclusions: Vec<String>,
}

impl Scanner {
    /// Creates a new scanner with the given exclusion patterns.
    #[must_use]
    pub const fn new(exclusions: Vec<String>) -> Self {
        Self {
            exclusions: Vec::new(),
            config_exclusions: exclusions,
        }
    }

    /// Loads exclusion patterns from `.gitignore` and `.dkvignore`
    /// files located at `root` and stores them as the scanner's
    /// pattern list.
    ///
    /// # Errors
    ///
    /// Returns [`std::io::Error`] if an ignore file cannot be read.
    pub fn load_ignore_files(&mut self, root: &Path) -> Result<()> {
        let patterns = load_ignore_files(root);
        self.exclusions = patterns;
        Ok(())
    }

    /// Returns `true` if an entry at `relative` (with `is_dir`) is excluded,
    /// combining ignore-file patterns (last-match-wins) with config
    /// exclusions (authoritative hard exclusions).
    fn is_excluded(&self, relative: &str, is_dir: bool) -> bool {
        if ignore::is_excluded(&self.exclusions, relative, is_dir) {
            return true;
        }

        self.config_exclusions.iter().any(|raw| {
            let pattern = ExclusionPattern {
                raw: raw.trim_end_matches('/').to_string(),
                is_negation: false,
                is_directory_only: raw.ends_with('/'),
            };
            matches_exclusion(&pattern, relative, is_dir)
        })
    }

    /// Walks the directory tree starting at `root`, yielding only
    /// entries that are not excluded by any pattern.
    ///
    /// Excluded directories are pruned, so their contents are never yielded.
    ///
    /// # Errors
    ///
    /// Returns [`std::io::Error`] if the root path cannot be read.
    #[allow(clippy::redundant_closure_for_method_calls)]
    pub fn walk(&self, root: &Path) -> Result<impl Iterator<Item = walkdir::DirEntry>> {
        let iter = WalkDir::new(root)
            .follow_links(false)
            .into_iter()
            .filter_entry(move |entry| {
                if entry.depth() == 0 || !entry.file_type().is_dir() {
                    return true;
                }

                let relative = entry
                    .path()
                    .strip_prefix(root)
                    .unwrap_or_else(|_| entry.path());
                let relative_str = relative.to_string_lossy();

                !self.is_excluded(&relative_str, true)
            })
            .filter_map(|e| e.ok())
            .filter(move |entry| {
                let relative = entry
                    .path()
                    .strip_prefix(root)
                    .unwrap_or_else(|_| entry.path());
                let relative_str = relative.to_string_lossy();
                let is_dir = entry.file_type().is_dir();

                !self.is_excluded(&relative_str, is_dir)
            });

        Ok(iter)
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::*;

    /// Collects the yielded entries as relative path strings.
    fn collect_relative_paths(
        root: &Path,
        entries: impl Iterator<Item = walkdir::DirEntry>,
    ) -> Vec<String> {
        entries
            .map(|entry| {
                entry
                    .path()
                    .strip_prefix(root)
                    .unwrap_or_else(|_| entry.path())
                    .to_string_lossy()
                    .into_owned()
            })
            .collect()
    }

    #[test]
    fn test_walk_prunes_gitignored_target_dir() {
        #[allow(clippy::unwrap_used)]
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();

        #[allow(clippy::unwrap_used)]
        fs::write(root.join(".gitignore"), "target/\n").unwrap();

        #[allow(clippy::unwrap_used)]
        fs::create_dir_all(root.join("target/debug")).unwrap();
        #[allow(clippy::unwrap_used)]
        fs::write(root.join("target/debug/app"), "bin").unwrap();
        #[allow(clippy::unwrap_used)]
        fs::write(root.join("Cargo.toml"), "[package]\n").unwrap();

        let mut scanner = Scanner::new(Vec::new());
        #[allow(clippy::unwrap_used)]
        scanner.load_ignore_files(root).unwrap();

        #[allow(clippy::unwrap_used)]
        let paths = collect_relative_paths(root, scanner.walk(root).unwrap());

        assert!(paths.iter().all(|p| !p.starts_with("target/")));
        assert!(paths.contains(&"Cargo.toml".to_string()));
    }

    #[test]
    fn test_walk_ignores_dkvignore_logs() {
        #[allow(clippy::unwrap_used)]
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();

        #[allow(clippy::unwrap_used)]
        fs::write(root.join(".dkvignore"), "*.log\n").unwrap();
        #[allow(clippy::unwrap_used)]
        fs::write(root.join("a.log"), "a").unwrap();
        #[allow(clippy::unwrap_used)]
        fs::create_dir_all(root.join("sub")).unwrap();
        #[allow(clippy::unwrap_used)]
        fs::write(root.join("sub/b.log"), "b").unwrap();
        #[allow(clippy::unwrap_used)]
        fs::write(root.join("c.txt"), "c").unwrap();

        let mut scanner = Scanner::new(Vec::new());
        #[allow(clippy::unwrap_used)]
        scanner.load_ignore_files(root).unwrap();

        #[allow(clippy::unwrap_used)]
        let paths = collect_relative_paths(root, scanner.walk(root).unwrap());

        assert!(!paths.contains(&"a.log".to_string()));
        assert!(!paths.contains(&"sub/b.log".to_string()));
        assert!(paths.contains(&"c.txt".to_string()));
    }

    #[test]
    fn test_walk_prunes_config_exclusions() {
        #[allow(clippy::unwrap_used)]
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();

        #[allow(clippy::unwrap_used)]
        fs::create_dir_all(root.join("node_modules/pkg")).unwrap();
        #[allow(clippy::unwrap_used)]
        fs::write(root.join("node_modules/pkg/index.js"), "mod").unwrap();
        #[allow(clippy::unwrap_used)]
        fs::create_dir_all(root.join("src")).unwrap();
        #[allow(clippy::unwrap_used)]
        fs::write(root.join("src/app.js"), "app").unwrap();

        let scanner = Scanner::new(vec!["node_modules".to_string()]);

        #[allow(clippy::unwrap_used)]
        let paths = collect_relative_paths(root, scanner.walk(root).unwrap());

        assert!(paths.iter().all(|p| !p.starts_with("node_modules/")));
        assert!(paths.contains(&"src/app.js".to_string()));
    }

    #[test]
    fn test_walk_negation_reincludes() {
        #[allow(clippy::unwrap_used)]
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();

        #[allow(clippy::unwrap_used)]
        fs::write(root.join(".gitignore"), "*.log\n!keep.log\n").unwrap();
        #[allow(clippy::unwrap_used)]
        fs::write(root.join("keep.log"), "keep").unwrap();
        #[allow(clippy::unwrap_used)]
        fs::write(root.join("other.log"), "other").unwrap();

        let mut scanner = Scanner::new(Vec::new());
        #[allow(clippy::unwrap_used)]
        scanner.load_ignore_files(root).unwrap();

        #[allow(clippy::unwrap_used)]
        let paths = collect_relative_paths(root, scanner.walk(root).unwrap());

        assert!(paths.contains(&"keep.log".to_string()));
        assert!(!paths.contains(&"other.log".to_string()));
    }

    #[test]
    fn test_walk_anchored_vendor_pattern() {
        #[allow(clippy::unwrap_used)]
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();

        #[allow(clippy::unwrap_used)]
        fs::write(root.join(".gitignore"), "/vendor\n").unwrap();
        #[allow(clippy::unwrap_used)]
        fs::create_dir_all(root.join("vendor")).unwrap();
        #[allow(clippy::unwrap_used)]
        fs::write(root.join("vendor/x.txt"), "x").unwrap();
        #[allow(clippy::unwrap_used)]
        fs::create_dir_all(root.join("src/vendor")).unwrap();
        #[allow(clippy::unwrap_used)]
        fs::write(root.join("src/vendor/y.txt"), "y").unwrap();

        let mut scanner = Scanner::new(Vec::new());
        #[allow(clippy::unwrap_used)]
        scanner.load_ignore_files(root).unwrap();

        #[allow(clippy::unwrap_used)]
        let paths = collect_relative_paths(root, scanner.walk(root).unwrap());

        assert!(
            paths
                .iter()
                .all(|p| p != "vendor" && !p.starts_with("vendor/"))
        );
        assert!(paths.contains(&"src/vendor".to_string()));
    }
}
