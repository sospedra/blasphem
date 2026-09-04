//! One version for everything. The root `Cargo.toml` `[workspace.package]`
//! defines it; every crate inherits it; this module writes it into the npm,
//! Python, Gradle, and standalone-crate manifests that cannot inherit.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use regex::Regex;
use thiserror::Error;

/// A manifest that mirrors the workspace version, with the pattern that finds its version line.
struct Mirror {
    path: PathBuf,
    pattern: Regex,
}

#[derive(Debug, Error)]
pub enum VersionsError {
    #[error("cannot read {path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: io::Error,
    },
    #[error("{0} has no [workspace.package] version")]
    NoWorkspaceVersion(PathBuf),
    #[error("{0} has no version field to mirror")]
    NoVersionField(PathBuf),
    #[error("{count} manifest(s) disagree with the workspace version {version}: {paths}")]
    Drift {
        count: usize,
        version: String,
        paths: String,
    },
}

#[derive(Debug, Clone, Copy)]
pub struct SyncReport {
    pub changed: usize,
    pub checked: usize,
}

/// The version in `[workspace.package]` of the root `Cargo.toml`.
pub fn workspace_version(root: &Path) -> Result<String, VersionsError> {
    let path = root.join("Cargo.toml");
    let text = read(&path)?;
    let pattern = Regex::new(r#"(?ms)^\[workspace\.package\]\n(?:[^\[].*\n)*?version = "([^"]+)""#)
        .expect("valid regex");
    pattern
        .captures(&text)
        .map(|captures| captures[1].to_owned())
        .ok_or(VersionsError::NoWorkspaceVersion(path))
}

fn mirrors(root: &Path) -> Result<Vec<Mirror>, VersionsError> {
    let json = || Regex::new(r#""version": "([^"]+)""#).expect("valid regex");
    let toml = || Regex::new(r#"(?m)^version = "([^"]+)""#).expect("valid regex");

    let mut found = vec![
        Mirror {
            path: root.join("crates/blasphem-python/Cargo.toml"),
            pattern: toml(),
        },
        Mirror {
            path: root.join("packages/python/pyproject.toml"),
            pattern: toml(),
        },
        Mirror {
            path: root.join("packages/python-packs/pyproject.toml"),
            pattern: toml(),
        },
        Mirror {
            path: root.join("packages/android/gradle.properties"),
            pattern: Regex::new(r"(?m)^VERSION_NAME=(.+)$").expect("valid regex"),
        },
    ];
    for package in [
        "javascript",
        "cli",
        "javascript-common",
        "node",
        "javascript-packs",
        "react-native",
    ] {
        found.push(Mirror {
            path: root.join("packages").join(package).join("package.json"),
            pattern: json(),
        });
    }
    for platform_root in ["packages/node/npm", "packages/cli/npm"] {
        let directory = root.join(platform_root);
        let Ok(entries) = fs::read_dir(&directory) else {
            continue;
        };
        let mut manifests: Vec<PathBuf> = entries
            .filter_map(Result::ok)
            .map(|entry| entry.path().join("package.json"))
            .filter(|path| path.is_file())
            .collect();
        manifests.sort();
        found.extend(manifests.into_iter().map(|path| Mirror {
            path,
            pattern: json(),
        }));
    }
    Ok(found
        .into_iter()
        .filter(|mirror| mirror.path.is_file())
        .collect())
}

/// Rewrites every mirror's version line. Idempotent.
///
/// # Errors
///
/// Returns an error when a file cannot be read or written, or lacks a version field.
pub fn sync_versions(root: &Path) -> Result<SyncReport, VersionsError> {
    let version = workspace_version(root)?;
    let mut changed = 0;
    let mirrors = mirrors(root)?;
    for mirror in &mirrors {
        let text = read(&mirror.path)?;
        let captures = mirror
            .pattern
            .captures(&text)
            .ok_or_else(|| VersionsError::NoVersionField(mirror.path.clone()))?;
        let current = captures.get(1).expect("one capture group");
        if current.as_str() == version {
            continue;
        }
        let mut updated = String::with_capacity(text.len());
        updated.push_str(&text[..current.start()]);
        updated.push_str(&version);
        updated.push_str(&text[current.end()..]);
        fs::write(&mirror.path, updated).map_err(|source| VersionsError::Io {
            path: mirror.path.clone(),
            source,
        })?;
        changed += 1;
    }
    Ok(SyncReport {
        changed,
        checked: mirrors.len(),
    })
}

/// Fails when any mirror disagrees with the workspace version. Writes nothing.
///
/// # Errors
///
/// Returns [`VersionsError::Drift`] naming every file that disagrees.
pub fn check_versions(root: &Path) -> Result<SyncReport, VersionsError> {
    let version = workspace_version(root)?;
    let mirrors = mirrors(root)?;
    let mut drifted = Vec::new();
    for mirror in &mirrors {
        let text = read(&mirror.path)?;
        let current = mirror
            .pattern
            .captures(&text)
            .ok_or_else(|| VersionsError::NoVersionField(mirror.path.clone()))?[1]
            .to_owned();
        if current != version {
            drifted.push(format!(
                "{} ({current})",
                mirror
                    .path
                    .strip_prefix(root)
                    .unwrap_or(&mirror.path)
                    .display()
            ));
        }
    }
    if !drifted.is_empty() {
        return Err(VersionsError::Drift {
            count: drifted.len(),
            version,
            paths: drifted.join(", "),
        });
    }
    Ok(SyncReport {
        changed: 0,
        checked: mirrors.len(),
    })
}

fn read(path: &Path) -> Result<String, VersionsError> {
    fs::read_to_string(path).map_err(|source| VersionsError::Io {
        path: path.to_owned(),
        source,
    })
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::{check_versions, workspace_version};

    fn root() -> &'static Path {
        Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../.."))
    }

    #[test]
    fn the_workspace_declares_one_version() {
        let version = workspace_version(root()).expect("workspace version");
        assert!(version.split('.').count() == 3, "{version}");
    }

    #[test]
    fn every_committed_manifest_carries_the_workspace_version() {
        let report = check_versions(root()).unwrap_or_else(|error| {
            panic!("{error}\nrun: cargo run -p blasphem-train -- sync-versions")
        });
        assert!(
            report.checked >= 10,
            "expected the npm, Python, and crate mirrors, found {}",
            report.checked
        );
    }
}
