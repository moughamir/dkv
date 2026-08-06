use std::fs;
use std::path::Path;

/// Represents a parsed exclusion pattern from a gitignore-style file.
#[derive(Debug, Clone)]
pub struct ExclusionPattern {
    /// The raw pattern string.
    pub raw: String,
    /// Whether this is a negation pattern (starts with `!`).
    pub is_negation: bool,
    /// Whether this pattern matches directories only (ends with `/`).
    pub is_directory_only: bool,
}

/// Reads exclusion patterns from `.gitignore` and `.dkvignore` files
/// located at the given root directory.
///
/// Returns a combined list of patterns from both files.
#[must_use]
pub fn load_ignore_files(root: &Path) -> Vec<ExclusionPattern> {
    let mut patterns = Vec::new();

    for filename in [".gitignore", ".dkvignore"] {
        let path = root.join(filename);
        if path.is_file()
            && let Ok(content) = fs::read_to_string(&path)
        {
            patterns.extend(parse_ignore_content(&content));
        }
    }

    patterns
}

/// Parses the content of an ignore file into a list of exclusion patterns.
fn parse_ignore_content(content: &str) -> Vec<ExclusionPattern> {
    content
        .lines()
        .filter_map(|line| {
            let trimmed = line.trim();

            if trimmed.is_empty() || trimmed.starts_with('#') {
                return None;
            }

            let is_negation = trimmed.starts_with('!');
            let raw = if is_negation {
                trimmed[1..].trim()
            } else {
                trimmed
            };

            if raw.is_empty() {
                return None;
            }

            let is_directory_only = raw.ends_with('/');
            let raw = if is_directory_only {
                &raw[..raw.len() - 1]
            } else {
                raw
            };

            if raw.is_empty() {
                return None;
            }

            Some(ExclusionPattern {
                raw: raw.to_string(),
                is_negation,
                is_directory_only,
            })
        })
        .collect()
}

/// Checks whether the given relative path matches an exclusion pattern.
///
/// `relative_path` must be a path relative to the ignore file's directory,
/// with forward slashes as separators.
///
/// Returns `true` if the path should be excluded.
#[must_use]
pub fn matches_exclusion(pattern: &ExclusionPattern, relative_path: &str, is_dir: bool) -> bool {
    if pattern.is_directory_only && !is_dir {
        return false;
    }

    let pattern_str = &pattern.raw;

    if let Some(suffix) = pattern_str.strip_prefix("**/") {
        return matches_suffix(suffix, relative_path)
            || relative_path.contains(&format!("/{suffix}"));
    }

    if pattern_str.contains('/') && !pattern_str.starts_with('/') {
        if relative_path.contains(pattern_str) {
            return true;
        }
        if let Some(pos) = relative_path.rfind('/') {
            let after_slash = &relative_path[pos + 1..];
            if matches_pattern(pattern_str, after_slash, false) {
                return true;
            }
        }
        return false;
    }

    let anchored = pattern_str.starts_with('/');
    let pattern_str = pattern_str.strip_prefix('/').unwrap_or(pattern_str);

    if anchored {
        // Anchored patterns match only against the full relative path and never
        // fall back to the basename.
        matches_pattern(pattern_str, relative_path, false)
    } else {
        matches_pattern(pattern_str, relative_path, true)
    }
}

/// Returns `true` if `relative_path` is excluded by `patterns`.
///
/// Gitignore semantics: the LAST matching pattern wins, so a later
/// negation (`!foo`) re-includes something matched by an earlier pattern.
#[must_use]
pub fn is_excluded(patterns: &[ExclusionPattern], relative_path: &str, is_dir: bool) -> bool {
    let mut excluded = false;

    for pattern in patterns {
        if matches_exclusion(pattern, relative_path, is_dir) {
            excluded = !pattern.is_negation;
        }
    }

    excluded
}

fn matches_pattern(pattern: &str, path: &str, match_basename: bool) -> bool {
    if pattern == "**" {
        return true;
    }

    // If the pattern contains no `/` and basename matching is enabled, match
    // against the basename so that `*.log` matches `src/debug.log`.
    if !pattern.contains('/') && match_basename {
        let basename = path.rsplit('/').next().unwrap_or(path);
        return match_wildcard(pattern, basename);
    }

    match_wildcard(pattern, path)
}

fn match_wildcard(pattern: &str, path: &str) -> bool {
    if let Some(pos) = pattern.find('*') {
        let prefix = &pattern[..pos];
        let suffix = &pattern[pos + 1..];

        if !path.starts_with(prefix) {
            return false;
        }

        let remaining = &path[prefix.len()..];

        if suffix.is_empty() {
            return true;
        }

        if let Some(suffix_pos) = remaining.find(suffix) {
            let between = &remaining[..suffix_pos];
            return !between.contains('/');
        }

        return false;
    }

    if let Some(pos) = pattern.find('?') {
        let prefix = &pattern[..pos];
        let suffix = &pattern[pos + 1..];

        if !path.starts_with(prefix) || !path.ends_with(suffix) {
            return false;
        }

        let middle_len = path.len() - prefix.len() - suffix.len();
        return middle_len == 1;
    }

    if let Some(prefix) = pattern.strip_suffix('*') {
        return path.starts_with(prefix);
    }

    if let Some(suffix) = pattern.strip_prefix('*') {
        return path.ends_with(suffix);
    }

    path == pattern
}

fn matches_suffix(suffix: &str, relative_path: &str) -> bool {
    if suffix.is_empty() {
        return true;
    }

    if let Some(pos) = relative_path.rfind(suffix) {
        let before = &relative_path[..pos];
        return before.is_empty() || before.ends_with('/');
    }

    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_ignore_content_skips_comments_and_blank() {
        let content = "# comment\n\n*.log\n";
        let patterns = parse_ignore_content(content);
        assert_eq!(patterns.len(), 1);
        assert_eq!(patterns[0].raw, "*.log");
        assert!(!patterns[0].is_negation);
    }

    #[test]
    fn test_parse_ignore_content_negation() {
        let content = "!important.log\n";
        let patterns = parse_ignore_content(content);
        assert_eq!(patterns.len(), 1);
        assert!(patterns[0].is_negation);
        assert_eq!(patterns[0].raw, "important.log");
    }

    #[test]
    fn test_parse_ignore_content_directory_only() {
        let content = "target/\n";
        let patterns = parse_ignore_content(content);
        assert_eq!(patterns.len(), 1);
        assert!(patterns[0].is_directory_only);
        assert_eq!(patterns[0].raw, "target");
    }

    #[test]
    fn test_matches_exclusion_wildcard() {
        let pattern = ExclusionPattern {
            raw: "*.log".to_string(),
            is_negation: false,
            is_directory_only: false,
        };
        assert!(matches_exclusion(&pattern, "debug.log", false));
        assert!(matches_exclusion(&pattern, "src/debug.log", false));
        assert!(!matches_exclusion(&pattern, "debug.txt", false));
    }

    #[test]
    fn test_matches_exclusion_directory_only() {
        let pattern = ExclusionPattern {
            raw: "target".to_string(),
            is_negation: false,
            is_directory_only: true,
        };
        assert!(matches_exclusion(&pattern, "target", true));
        assert!(!matches_exclusion(&pattern, "target", false));
        assert!(!matches_exclusion(&pattern, "target/file.txt", false));
    }

    #[test]
    fn test_matches_exclusion_leading_slash() {
        let pattern = ExclusionPattern {
            raw: "/Cargo.toml".to_string(),
            is_negation: false,
            is_directory_only: false,
        };
        assert!(matches_exclusion(&pattern, "Cargo.toml", false));
        assert!(!matches_exclusion(&pattern, "sub/Cargo.toml", false));
    }

    #[test]
    fn test_matches_exclusion_double_star() {
        let pattern = ExclusionPattern {
            raw: "**/target".to_string(),
            is_negation: false,
            is_directory_only: false,
        };
        assert!(matches_exclusion(&pattern, "target", true));
        assert!(matches_exclusion(&pattern, "sub/target", true));
        assert!(matches_exclusion(&pattern, "a/b/target", true));
    }

    #[test]
    fn test_matches_exclusion_question_mark() {
        let pattern = ExclusionPattern {
            raw: "?.log".to_string(),
            is_negation: false,
            is_directory_only: false,
        };
        assert!(matches_exclusion(&pattern, "a.log", false));
        assert!(!matches_exclusion(&pattern, "ab.log", false));
    }

    #[test]
    fn test_load_ignore_files() {
        #[allow(clippy::unwrap_used)]
        let dir = tempfile::tempdir().unwrap();
        let gitignore = dir.path().join(".gitignore");
        #[allow(clippy::unwrap_used)]
        fs::write(&gitignore, "*.log\ntarget/\n").unwrap();

        let patterns = load_ignore_files(dir.path());
        assert_eq!(patterns.len(), 2);
        assert_eq!(patterns[0].raw, "*.log");
        assert!(patterns[1].is_directory_only);
        assert_eq!(patterns[1].raw, "target");
    }

    #[test]
    fn test_load_ignore_files_dkvignore() {
        #[allow(clippy::unwrap_used)]
        let dir = tempfile::tempdir().unwrap();
        let dkvignore = dir.path().join(".dkvignore");
        #[allow(clippy::unwrap_used)]
        fs::write(&dkvignore, "*.tmp\n").unwrap();

        let patterns = load_ignore_files(dir.path());
        assert_eq!(patterns.len(), 1);
        assert_eq!(patterns[0].raw, "*.tmp");
    }

    #[test]
    fn test_is_excluded_last_match_wins() {
        let patterns = vec![
            ExclusionPattern {
                raw: "*.log".to_string(),
                is_negation: false,
                is_directory_only: false,
            },
            ExclusionPattern {
                raw: "keep.log".to_string(),
                is_negation: true,
                is_directory_only: false,
            },
        ];

        assert!(!is_excluded(&patterns, "keep.log", false));
        assert!(is_excluded(&patterns, "other.log", false));
    }

    #[test]
    fn test_is_excluded_no_match_is_included() {
        let patterns = vec![ExclusionPattern {
            raw: "*.log".to_string(),
            is_negation: false,
            is_directory_only: false,
        }];

        assert!(!is_excluded(&patterns, "notes.txt", false));
    }

    #[test]
    fn test_matches_exclusion_leading_slash_does_not_match_subdir() {
        let pattern = ExclusionPattern {
            raw: "/foo".to_string(),
            is_negation: false,
            is_directory_only: false,
        };

        assert!(matches_exclusion(&pattern, "foo", false));
        assert!(!matches_exclusion(&pattern, "sub/foo", false));
    }
}
