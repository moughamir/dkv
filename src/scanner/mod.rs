use std::cell::Cell;
use std::path::Path;
use std::time::Duration;

use walkdir::WalkDir;

use crate::error::Result;
use crate::types::ScanStatistics;

mod ignore;

pub use ignore::{ExclusionPattern, is_excluded, load_ignore_files, matches_exclusion};

/// Directories ignored by default on every scan, in addition to any
/// user-configured exclusions.
pub const DEFAULT_IGNORES: &[&str] = &[
    ".git",
    "target",
    "node_modules",
    "dist",
    "build",
    "coverage",
    ".cache",
    ".next",
    ".nuxt",
    ".svelte-kit",
    "vendor",
];

/// Filesystem scanner that supports configurable exclusions and
/// `.gitignore` / `.dkvignore` patterns.
pub struct Scanner {
    exclusions: Vec<ExclusionPattern>,
    config_exclusions: Vec<String>,
    directories_scanned: Cell<usize>,
    files_scanned: Cell<usize>,
    ignored_directories: Cell<usize>,
}

impl Scanner {
    /// Creates a new scanner with the given exclusion patterns.
    #[must_use]
    pub const fn new(exclusions: Vec<String>) -> Self {
        Self {
            exclusions: Vec::new(),
            config_exclusions: exclusions,
            directories_scanned: Cell::new(0),
            files_scanned: Cell::new(0),
            ignored_directories: Cell::new(0),
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
    /// combining ignore-file patterns (last-match-wins), the default ignore
    /// list, and config exclusions (authoritative hard exclusions).
    fn is_excluded(&self, relative: &str, is_dir: bool) -> bool {
        if ignore::is_excluded(&self.exclusions, relative, is_dir) {
            return true;
        }

        let default_excluded = DEFAULT_IGNORES.iter().any(|name| {
            let pattern = ExclusionPattern {
                raw: (*name).to_string(),
                is_negation: false,
                is_directory_only: false,
            };
            matches_exclusion(&pattern, relative, is_dir)
        });
        if default_excluded {
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

                if self.is_excluded(&relative_str, true) {
                    self.ignored_directories
                        .set(self.ignored_directories.get() + 1);
                    false
                } else {
                    self.directories_scanned
                        .set(self.directories_scanned.get() + 1);
                    true
                }
            })
            .filter_map(|e| e.ok())
            .filter(move |entry| {
                let relative = entry
                    .path()
                    .strip_prefix(root)
                    .unwrap_or_else(|_| entry.path());
                let relative_str = relative.to_string_lossy();
                let is_dir = entry.file_type().is_dir();

                let passes = !self.is_excluded(&relative_str, is_dir);
                if passes {
                    if is_dir {
                        self.directories_scanned
                            .set(self.directories_scanned.get() + 1);
                    } else {
                        self.files_scanned.set(self.files_scanned.get() + 1);
                    }
                }
                passes
            });

        Ok(iter)
    }

    /// Returns the traversal statistics collected so far by this scanner.
    ///
    /// `elapsed` is always [`Duration::ZERO`]; the CLI sets it after the scan
    /// completes.
    #[must_use]
    pub const fn statistics(&self) -> ScanStatistics {
        ScanStatistics {
            directories_scanned: self.directories_scanned.get(),
            files_scanned: self.files_scanned.get(),
            ignored_directories: self.ignored_directories.get(),
            elapsed: Duration::ZERO,
        }
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
        // `vendor` is also in DEFAULT_IGNORES, so this test uses `output`
        // to verify that anchored patterns match only at the root.
        #[allow(clippy::unwrap_used)]
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();

        #[allow(clippy::unwrap_used)]
        fs::write(root.join(".gitignore"), "/output\n").unwrap();
        #[allow(clippy::unwrap_used)]
        fs::create_dir_all(root.join("output")).unwrap();
        #[allow(clippy::unwrap_used)]
        fs::write(root.join("output/x.txt"), "x").unwrap();
        #[allow(clippy::unwrap_used)]
        fs::create_dir_all(root.join("src/output")).unwrap();
        #[allow(clippy::unwrap_used)]
        fs::write(root.join("src/output/y.txt"), "y").unwrap();

        let mut scanner = Scanner::new(Vec::new());
        #[allow(clippy::unwrap_used)]
        scanner.load_ignore_files(root).unwrap();

        #[allow(clippy::unwrap_used)]
        let paths = collect_relative_paths(root, scanner.walk(root).unwrap());

        assert!(
            paths
                .iter()
                .all(|p| p != "output" && !p.starts_with("output/"))
        );
        assert!(paths.contains(&"src/output".to_string()));
    }

    #[test]
    fn test_walk_default_ignores_prune_dist() {
        #[allow(clippy::unwrap_used)]
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();

        #[allow(clippy::unwrap_used)]
        fs::create_dir_all(root.join("dist")).unwrap();
        #[allow(clippy::unwrap_used)]
        fs::write(root.join("dist/bundle.js"), "bundle").unwrap();
        #[allow(clippy::unwrap_used)]
        fs::create_dir_all(root.join("src")).unwrap();
        #[allow(clippy::unwrap_used)]
        fs::write(root.join("src/app.js"), "app").unwrap();
        #[allow(clippy::unwrap_used)]
        fs::create_dir_all(root.join("coverage")).unwrap();
        #[allow(clippy::unwrap_used)]
        fs::write(root.join("coverage/lcov.info"), "lcov").unwrap();

        // Empty config exclusions prove DEFAULT_IGNORES does the pruning.
        let scanner = Scanner::new(Vec::new());

        #[allow(clippy::unwrap_used)]
        let paths = collect_relative_paths(root, scanner.walk(root).unwrap());

        assert!(paths.iter().all(|p| !p.starts_with("dist/")));
        assert!(paths.iter().all(|p| !p.starts_with("coverage/")));
        assert!(paths.contains(&"src/app.js".to_string()));

        let statistics = scanner.statistics();
        assert!(statistics.ignored_directories >= 2);
        assert!(statistics.directories_scanned >= 2);
    }
}
