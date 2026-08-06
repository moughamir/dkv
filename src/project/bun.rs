//! Bun-specific manifest and lockfile handling.

use std::{collections::BTreeMap, fs, path::Path};

use serde::Deserialize;

use crate::{error::Result, project::npm::split_name_version, types::LockedPackage};

/// Raw `bun.lock` structure (relevant subset).
#[derive(Debug, Deserialize)]
struct BunLock {
    #[serde(default)]
    packages: BTreeMap<String, BunPackage>,
}

/// A single entry in a `bun.lock` `packages` map.
#[derive(Debug, Deserialize)]
struct BunPackage {
    version: Option<String>,
}

/// Parses a `bun.lock` (text JSON) into the packages it locks.
///
/// # Errors
///
/// Returns [`crate::error::DkvError`] if the file cannot be read or is not
/// valid JSON.
pub fn parse_bun_lock(path: &Path) -> Result<Vec<LockedPackage>> {
    let text = fs::read_to_string(path)?;
    let lock: BunLock = serde_json::from_str(&text)?;
    let mut packages = Vec::new();
    for (raw_key, entry) in lock.packages {
        let key = raw_key.strip_prefix('/').unwrap_or(&raw_key);
        let (name, key_version) =
            split_name_version(key).unwrap_or_else(|| (key.to_string(), String::new()));
        let version = entry.version.unwrap_or(key_version);
        packages.push(LockedPackage { name, version });
    }
    Ok(packages)
}

#[cfg(test)]
mod tests {
    use std::fs;

    use tempfile::tempdir;

    use super::parse_bun_lock;
    use crate::types::LockedPackage;

    #[test]
    fn parses_text_bun_lock() {
        #[allow(clippy::unwrap_used)]
        let dir = tempdir().unwrap();
        let path = dir.path().join("bun.lock");
        #[allow(clippy::unwrap_used)]
        fs::write(
            &path,
            r#"{
                "lockfileVersion": 1,
                "packages": {
                    "/foo@1.2.3": { "version": "1.2.3" },
                    "@scope/pkg@3.0.0": { "version": "3.0.0" },
                    "bar": { "version": "2.0.0" }
                }
            }"#,
        )
        .unwrap();
        #[allow(clippy::unwrap_used)]
        let packages = parse_bun_lock(&path).unwrap();
        assert_eq!(
            packages,
            vec![
                LockedPackage {
                    name: "foo".to_string(),
                    version: "1.2.3".to_string()
                },
                LockedPackage {
                    name: "@scope/pkg".to_string(),
                    version: "3.0.0".to_string(),
                },
                LockedPackage {
                    name: "bar".to_string(),
                    version: "2.0.0".to_string()
                },
            ]
        );
    }
}
