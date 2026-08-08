//! Transaction support and file staging for the storage layer.

use std::fs;
use std::path::Path;

use crate::storage::errors::{Result, StorageError};

/// Stages a content file into `tmp_dir` and renames it to `dest` if not already present.
/// Returns whether the file was newly written.
///
/// # Errors
///
/// Returns [`StorageError::Io`] if directory creation, file write, or rename fails.
pub fn stage_content(tmp_dir: &Path, dest: &Path, content: &[u8]) -> Result<bool> {
    if dest.exists() {
        return Ok(false);
    }
    fs::create_dir_all(tmp_dir)?;
    let tmp_file = tmp_dir.join(format!("staged-{}", unique_suffix()));
    fs::write(&tmp_file, content)?;
    if let Err(e) = fs::rename(&tmp_file, dest) {
        let _ = fs::remove_file(&tmp_file);
        return Err(StorageError::Io(e));
    }
    Ok(true)
}

/// Generates a unique hex suffix for temp staging files.
fn unique_suffix() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    format!("{nanos:016x}")
}
