use std::{fs, path::Path};

/// Old first-party identifiers that must not survive in active source.
const RETIRED_NAMES: [&str; 8] = [
    "toxcheck",
    "toxtrain",
    "toxbench",
    "toxcheck-wasm",
    "toxcheck_wasm",
    "eldc",
    "ELDC",
    "import-eldc",
];

/// Directories that hold generated output, history, or third-party records.
const SKIPPED_DIRECTORIES: [&str; 6] = [
    ".git",
    ".superpowers",
    "target",
    "node_modules",
    "vendor",
    "dist",
];

/// Files that keep the upstream name as attribution or as a pinned record.
const ATTRIBUTION_FILES: [&str; 6] = [
    "crates/blasphem-language/UPSTREAM.md",
    "crates/blasphem-language/FORMAT.md",
    "NOTICE",
    "packages/blasphem/NOTICE",
    "crates/blasphem-language/tools/build-c-oracle.sh",
    "tests/rename_contract.rs",
];

fn project_root() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
}

fn is_scanned(relative: &str) -> bool {
    if ATTRIBUTION_FILES.contains(&relative) {
        return false;
    }
    if relative.starts_with("docs/") || relative.starts_with("reports/") {
        return false;
    }
    let extension = Path::new(relative).extension().and_then(|value| value.to_str());
    matches!(
        extension,
        Some("rs" | "toml" | "md" | "json" | "mjs" | "js" | "ts" | "sh" | "yml" | "html")
    )
}

fn collect(directory: &Path, root: &Path, found: &mut Vec<String>) {
    let entries = fs::read_dir(directory).expect("readable directory");
    for entry in entries {
        let entry = entry.expect("readable entry");
        let path = entry.path();
        let name = entry.file_name();
        let name = name.to_str().expect("UTF-8 file name");
        if path.is_dir() {
            if !SKIPPED_DIRECTORIES.contains(&name) {
                collect(&path, root, found);
            }
            continue;
        }
        let relative = path
            .strip_prefix(root)
            .expect("path inside the project root")
            .to_str()
            .expect("UTF-8 relative path")
            .to_owned();
        if !is_scanned(&relative) {
            continue;
        }
        let Ok(text) = fs::read_to_string(&path) else {
            continue;
        };
        for retired in RETIRED_NAMES {
            if text.contains(retired) {
                found.push(format!("{relative}: {retired}"));
            }
        }
    }
}

#[test]
fn active_source_uses_only_blasphem_names() {
    let root = project_root();
    let mut found = Vec::new();
    collect(root, root, &mut found);
    found.sort();
    assert!(
        found.is_empty(),
        "retired first-party names remain in active source:\n{}",
        found.join("\n")
    );
}
