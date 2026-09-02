//! Writes the per-language packs, detect slices, and the manifest that
//! `@blasphem/packs` ships.

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

/// The `formatVersion` the JavaScript core accepts.
pub const PACKS_MANIFEST_FORMAT_VERSION: u32 = 1;

#[derive(Debug, Clone)]
pub struct PackOptions {
    pub model_manifest: PathBuf,
    pub model_root: PathBuf,
    pub language_model: PathBuf,
    pub hurtlex_root: PathBuf,
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
    fs::create_dir_all(&options.output).map_err(|source| io_error(&options.output, source))?;
    clear_previous_output(&options.output)?;

    let mut files = BTreeMap::new();
    let mut bytes = 0_usize;
    for language in Language::ALL {
        let pack = pack_bytes(options, &manifest, language)?;
        let slice = slices
            .get(&language)
            .ok_or(PackWriteError::MissingSlice(language))?;
        bytes += write_file(
            &options.output,
            &pack_file_name(language),
            &pack,
            &mut files,
        )?;
        bytes += write_file(
            &options.output,
            &detect_file_name(language),
            slice,
            &mut files,
        )?;
    }

    let packs_manifest = PacksManifest {
        format_version: PACKS_MANIFEST_FORMAT_VERSION,
        files,
    };
    let mut json = serde_json::to_vec_pretty(&packs_manifest)?;
    json.push(b'\n');
    let manifest_path = options.output.join("manifest.json");
    fs::write(&manifest_path, &json).map_err(|source| io_error(&manifest_path, source))?;
    let (module, declaration) = files_module(&packs_manifest);
    let module_path = options.output.join("files.js");
    fs::write(&module_path, &module).map_err(|source| io_error(&module_path, source))?;
    let declaration_path = options.output.join("files.d.ts");
    fs::write(&declaration_path, &declaration)
        .map_err(|source| io_error(&declaration_path, source))?;

    Ok(PackReport {
        locales: Language::ALL.len(),
        files: packs_manifest.files.len() + 3,
        bytes: bytes + json.len() + module.len() + declaration.len(),
    })
}

/// A module that names every shipped file with `new URL(literal, import.meta.url)`.
///
/// Node loads packs through it, so file tracers such as `@vercel/nft` see the
/// exact files a deployment needs without any configuration. The module is
/// Node-only; the browser never imports it.
fn files_module(manifest: &PacksManifest) -> (String, String) {
    let mut module = String::new();
    module.push_str("// Written by `blasphem-train pack`. Do not edit.\n");
    module.push_str(
        "// Every entry is a literal `new URL` so deployment tracers include the file.\n",
    );
    module.push_str("export const MANIFEST = new URL(\"./manifest.json\", import.meta.url);\n");
    module.push_str("export const FILES = {\n");
    module.push_str("  \"manifest.json\": MANIFEST,\n");
    for name in manifest.files.keys() {
        module.push_str(&format!(
            "  \"{name}\": new URL(\"./{name}\", import.meta.url),\n"
        ));
    }
    module.push_str("};\n");
    let declaration = "export declare const MANIFEST: URL;\nexport declare const FILES: Readonly<Record<string, URL>>;\n".to_owned();
    (module, declaration)
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
    let lexicon_path = options.hurtlex_root.join(format!("{storage}.tsv"));
    let lexicon = match &entry.hurtlex_sha256 {
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

fn write_file(
    output: &Path,
    name: &str,
    bytes: &[u8],
    files: &mut BTreeMap<String, PackedFile>,
) -> Result<usize, PackWriteError> {
    let path = output.join(name);
    fs::write(&path, bytes).map_err(|source| io_error(&path, source))?;
    files.insert(
        name.to_owned(),
        PackedFile {
            bytes: bytes.len(),
            sha256: hex_digest(bytes),
        },
    );
    Ok(bytes.len())
}

/// Removes the files a previous run wrote, so the manifest describes the directory exactly.
fn clear_previous_output(output: &Path) -> Result<(), PackWriteError> {
    let entries = fs::read_dir(output).map_err(|source| io_error(output, source))?;
    for entry in entries {
        let path = entry.map_err(|source| io_error(output, source))?.path();
        let stale = path
            .file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| {
                name == "manifest.json" || name.ends_with(".pack") || name.ends_with(".detect")
            });
        if stale {
            fs::remove_file(&path).map_err(|source| io_error(&path, source))?;
        }
    }
    Ok(())
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
