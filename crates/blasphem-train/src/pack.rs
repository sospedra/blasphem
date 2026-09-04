//! Writes the canonical per-language binary packs, detect slices, and manifest.

use std::collections::BTreeMap;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::str::FromStr;

use blasphem::{Language, PackInput, detect_file_name, encode_pack, pack_file_name};
use serde::Serialize;
use sha2::{Digest, Sha256};
use thiserror::Error;

use crate::model_manifest::{ModelManifest, ModelSetError, parse_model_manifest};

/// The shared pack manifest format version.
pub const PACKS_MANIFEST_FORMAT_VERSION: u32 = 1;

#[derive(Debug, Clone)]
pub struct PackOptions {
    pub model_manifest: PathBuf,
    pub model_root: PathBuf,
    pub language_model: PathBuf,
    pub lexicon_root: PathBuf,
    pub output: PathBuf,
}

#[derive(Debug, Serialize)]
pub struct PacksManifest {
    #[serde(rename = "formatVersion")]
    pub format_version: u32,
    pub files: BTreeMap<String, PackedFile>,
}

#[derive(Debug, Serialize)]
pub struct PackedFile {
    pub bytes: usize,
    pub sha256: String,
}

#[derive(Debug, Clone, Copy)]
pub struct PackReport {
    pub locales: usize,
    pub files: usize,
    pub bytes: usize,
}

#[derive(Debug, Error)]
pub enum PackWriteError {
    #[error("cannot access {path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: io::Error,
    },
    #[error("invalid model manifest: {0}")]
    Manifest(#[from] ModelSetError),
    #[error("{path} has sha256 {actual}, the model manifest records {expected}")]
    DigestMismatch {
        path: PathBuf,
        expected: String,
        actual: String,
    },
    #[error("the model manifest has no entry for {0}")]
    MissingEntry(Language),
    #[error("cannot slice the language model: {0}")]
    Slices(#[from] blasphem_language::ModelError),
    #[error("the language model has no slice for {0}")]
    MissingSlice(Language),
    #[error("cannot encode the packs manifest: {0}")]
    Json(#[from] serde_json::Error),
}

/// Writes `{code}.pack`, `{code}.detect` for every language, then `manifest.json`.
///
/// Every artifact and lexicon is checked against the model manifest first, so
/// a pack never carries bytes the reproduction pipeline did not record.
///
/// # Errors
///
/// Returns an error for an unreadable input, a digest that disagrees with the
/// model manifest, or a language without a manifest entry or slice.
pub fn write_packs(options: &PackOptions) -> Result<PackReport, PackWriteError> {
    let manifest = read_model_manifest(&options.model_manifest)?;
    let slices = slices_by_language(&read(&options.language_model)?)?;
    let artifacts = collect_artifacts(options, &manifest, &slices)?;
    let files = artifacts
        .iter()
        .map(|(name, bytes)| {
            (
                name.clone(),
                PackedFile {
                    bytes: bytes.len(),
                    sha256: hex_digest(bytes),
                },
            )
        })
        .collect();
    let packs_manifest = PacksManifest {
        format_version: PACKS_MANIFEST_FORMAT_VERSION,
        files,
    };
    let mut json = serde_json::to_vec_pretty(&packs_manifest)?;
    json.push(b'\n');
    publish_artifacts(&options.output, &artifacts, &json)?;
    Ok(PackReport {
        locales: Language::ALL.len(),
        files: artifacts.len() + 1,
        bytes: artifacts.values().map(Vec::len).sum::<usize>() + json.len(),
    })
}

fn collect_artifacts(
    options: &PackOptions,
    manifest: &ModelManifest,
    slices: &BTreeMap<Language, Vec<u8>>,
) -> Result<BTreeMap<String, Vec<u8>>, PackWriteError> {
    let pairs = Language::ALL
        .into_iter()
        .map(|language| {
            let pack = pack_bytes(options, manifest, language)?;
            let slice = slices
                .get(&language)
                .ok_or(PackWriteError::MissingSlice(language))?;
            Ok([
                (pack_file_name(language), pack),
                (detect_file_name(language), slice.clone()),
            ])
        })
        .collect::<Result<Vec<_>, PackWriteError>>()?;
    Ok(pairs.into_iter().flatten().collect())
}

fn publish_artifacts(
    output: &Path,
    artifacts: &BTreeMap<String, Vec<u8>>,
    manifest: &[u8],
) -> Result<(), PackWriteError> {
    fs::create_dir_all(output).map_err(|source| io_error(output, source))?;
    let staged = tempfile::Builder::new()
        .prefix(".packs-")
        .tempdir_in(output)
        .map_err(|source| io_error(output, source))?;
    for (name, bytes) in artifacts {
        write_file(&staged.path().join(name), bytes)?;
    }
    write_file(&staged.path().join("manifest.json"), manifest)?;
    for name in artifacts
        .keys()
        .map(String::as_str)
        .chain(["manifest.json"])
    {
        fs::rename(staged.path().join(name), output.join(name))
            .map_err(|source| io_error(&output.join(name), source))?;
    }
    clear_previous_output(output, artifacts)
}

fn read_model_manifest(path: &Path) -> Result<ModelManifest, PackWriteError> {
    let file = fs::File::open(path).map_err(|source| io_error(path, source))?;
    Ok(parse_model_manifest(io::BufReader::new(file))?)
}

fn slices_by_language(model: &[u8]) -> Result<BTreeMap<Language, Vec<u8>>, PackWriteError> {
    let mut slices = BTreeMap::new();
    for (slice_language, bytes) in blasphem_language::slice::write_slices(model)? {
        let language = Language::from_str(slice_language.code())
            .expect("every language model profile is a supported language");
        slices.insert(language, bytes);
    }
    Ok(slices)
}

fn pack_bytes(
    options: &PackOptions,
    manifest: &ModelManifest,
    language: Language,
) -> Result<Vec<u8>, PackWriteError> {
    let entry = manifest
        .entries
        .iter()
        .find(|entry| entry.language == language)
        .ok_or(PackWriteError::MissingEntry(language))?;
    let artifact_path = options.model_root.join(&entry.artifact_relative_path);
    let artifact = read_verified(&artifact_path, &entry.artifact_sha256.to_string())?;
    let storage = language.storage_code();
    let lexicon_path = options.lexicon_root.join(format!("{storage}.tsv"));
    let lexicon = match &entry.lexicon_sha256 {
        Some(expected) => read_verified(&lexicon_path, &expected.to_string())?,
        None => read(&lexicon_path)?,
    };
    Ok(encode_pack(&PackInput {
        language,
        rule_pack_version: entry.rule_pack_version,
        artifact: &artifact,
        lexicon: &lexicon,
    }))
}

fn read_verified(path: &Path, expected: &str) -> Result<Vec<u8>, PackWriteError> {
    let bytes = read(path)?;
    let actual = hex_digest(&bytes);
    if actual != expected {
        return Err(PackWriteError::DigestMismatch {
            path: path.to_owned(),
            expected: expected.to_owned(),
            actual,
        });
    }
    Ok(bytes)
}

fn write_file(path: &Path, bytes: &[u8]) -> Result<(), PackWriteError> {
    fs::write(path, bytes).map_err(|source| io_error(path, source))
}

fn clear_previous_output(
    output: &Path,
    artifacts: &BTreeMap<String, Vec<u8>>,
) -> Result<(), PackWriteError> {
    for entry in fs::read_dir(output).map_err(|source| io_error(output, source))? {
        let path = entry.map_err(|source| io_error(output, source))?.path();
        let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
            continue;
        };
        if is_generated_file(name) && !artifacts.contains_key(name) {
            fs::remove_file(&path).map_err(|source| io_error(&path, source))?;
        }
    }
    Ok(())
}

fn is_generated_file(name: &str) -> bool {
    name.ends_with(".pack")
        || name.ends_with(".detect")
        || matches!(name, "files.js" | "files.d.ts")
}

fn read(path: &Path) -> Result<Vec<u8>, PackWriteError> {
    fs::read(path).map_err(|source| io_error(path, source))
}

fn io_error(path: &Path, source: io::Error) -> PackWriteError {
    PackWriteError::Io {
        path: path.to_owned(),
        source,
    }
}

fn hex_digest(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
