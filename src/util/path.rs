//! Path utilities.

use std::path::{Path, PathBuf};

/// Expands leading `~` in path to user home directory if present.
#[must_use]
pub fn expand_home(path: &Path) -> PathBuf {
    let path_str = path.to_string_lossy();
    if (path_str.starts_with("~/") || path_str == "~")
        && let Some(base) = directories::BaseDirs::new()
    {
        let home = base.home_dir();
        if path_str == "~" {
            return home.to_path_buf();
        }
        return home.join(&path_str[2..]);
    }
    path.to_path_buf()
}
