use std::{collections::VecDeque, fmt::Write as _, fs, fs::File, process::Command};

use tempfile::tempdir;
use blasphem_train::{
    TextDetoxAcquisitionError, TextDetoxFetchError, TextDetoxHttpClient, TextDetoxHttpResponse,
    TextDetoxTransportError, acquire_textdetox, datasets::textdetox::TextDetoxError,
    fetch_textdetox, textdetox_rows_url,
};

#[test]
fn observe_reads_the_catalog_and_refuses_overwrite() {
    let directory = tempdir().expect("temporary directory");
    let catalog = directory.path().join("catalog.json");
    let observation = directory.path().join("observation");
    fs::write(
        &catalog,
        r#"{"schema_version":"source-catalog-v1","sources":[]}"#,
    )
    .expect("write catalog");

    let first = Command::new(env!("CARGO_BIN_EXE_blasphem-train"))
        .args([
            "observe",
            "--source-catalog",
            catalog.to_str().expect("UTF-8 path"),
            "--output",
            observation.to_str().expect("UTF-8 path"),
        ])
        .output()
        .expect("run observe");
    assert!(first.status.success(), "{:?}", first.stderr);

    let second = Command::new(env!("CARGO_BIN_EXE_blasphem-train"))
        .args([
            "observe",
            "--source-catalog",
            catalog.to_str().expect("UTF-8 path"),
            "--output",
            observation.to_str().expect("UTF-8 path"),
        ])
        .output()
        .expect("run observe");
    assert!(!second.status.success());
}

#[test]
fn freeze_sources_requires_review_and_refuses_overwrite() {
    let directory = tempdir().expect("temporary directory");
    let observation = directory.path().join("observation.json");
    fs::write(
        &observation,
        r#"{"schema_version":"source-observation-v1","sources":[]}"#,
    )
    .expect("write observation");
    let lock = directory.path().join("source-lock.json");
    let rejected = Command::new(env!("CARGO_BIN_EXE_blasphem-train"))
        .args([
            "freeze-sources",
            "--observation",
            observation
                .join("source-observation-v1.json")
                .to_str()
                .expect("UTF-8 path"),
            "--output",
            lock.to_str().expect("UTF-8 path"),
        ])
        .output()
        .expect("run freeze");
    assert!(!rejected.status.success());
}

#[test]
fn freeze_sources_writes_the_reviewed_lock_and_refuses_overwrite() {
    let directory = tempdir().expect("temporary directory");
    let observation = directory.path().join("observation.json");
    let lock = directory.path().join("source-lock.json");
    write_catalog_observation(&observation);

    let first = blasphem_train_command(&[
        "freeze-sources",
        "--observation",
        observation.to_str().expect("UTF-8 path"),
        "--reviewed",
        "--output",
        lock.to_str().expect("UTF-8 path"),
    ]);
    assert!(first.status.success(), "{:?}", first.stderr);
    assert!(lock.is_file());

    let second = blasphem_train_command(&[
        "freeze-sources",
        "--observation",
        observation.to_str().expect("UTF-8 path"),
        "--reviewed",
        "--output",
        lock.to_str().expect("UTF-8 path"),
    ]);
    assert!(!second.status.success());
}

#[test]
fn observe_refuses_an_existing_output_before_a_source_request() {
    let directory = tempdir().expect("temporary directory");
    let catalog = directory.path().join("catalog.json");
    let output = directory.path().join("observation");
    fs::create_dir(&output).expect("create output");
    fs::write(
        &catalog,
        one_source_catalog("http://127.0.0.1:1/unreachable"),
    )
    .expect("write catalog");

    let result = blasphem_train_command(&[
        "observe",
        "--source-catalog",
        catalog.to_str().expect("UTF-8 path"),
        "--output",
        output.to_str().expect("UTF-8 path"),
    ]);

    assert!(!result.status.success());
    assert!(
        String::from_utf8(result.stderr)
            .expect("UTF-8 error")
            .contains("source output already exists")
    );
}

#[test]
fn acquire_refuses_an_existing_output_before_a_source_request() {
    let directory = tempdir().expect("temporary directory");
    let lock = directory.path().join("source-lock.json");
    let output = directory.path().join("raw");
    fs::create_dir(&output).expect("create output");
    fs::write(&lock, one_source_lock("http://127.0.0.1:1/unreachable")).expect("write lock");

    let result = blasphem_train_command(&[
        "acquire",
        "--source-lock",
        lock.to_str().expect("UTF-8 path"),
        "--output",
        output.to_str().expect("UTF-8 path"),
    ]);

    assert!(!result.status.success());
    assert!(
        String::from_utf8(result.stderr)
            .expect("UTF-8 error")
            .contains("source output already exists")
    );
}

#[test]
fn acquire_rejects_a_missing_textdetox_digest_before_http() {
    let directory = tempdir().expect("temporary directory");
    let lock = directory.path().join("source-lock.json");
    let output = directory.path().join("raw");
    let revision = blasphem_train::TEXTDETOX_REVISION;
    let url = format!(
        "https://huggingface.co/datasets/textdetox/multilingual_toxicity_dataset/resolve/{revision}/data/en-00000-of-00001.parquet"
    );
    fs::write(&lock, one_textdetox_source_lock(&url, revision)).expect("write lock");

    let result = blasphem_train_command(&[
        "acquire",
        "--source-lock",
        lock.to_str().expect("UTF-8 path"),
        "--output",
        output.to_str().expect("UTF-8 path"),
    ]);

    assert!(!result.status.success());
    assert!(
        String::from_utf8(result.stderr)
            .expect("UTF-8 error")
            .contains("has no Parquet download digest")
    );
    assert!(!output.exists());
}

#[test]
fn eval_rejects_the_removed_include_inclusive_argument() {
    let output = Command::new(env!("CARGO_BIN_EXE_blasphem-train"))
        .args(["eval", "--input", "missing.tsv", "--include-inclusive"])
        .output()
        .expect("run blasphem-train");

    assert!(!output.status.success());
    let stderr = String::from_utf8(output.stderr).expect("UTF-8 error");
    assert!(stderr.contains("unexpected argument '--include-inclusive'"));
}

const EN_LEXICON: &str = concat!(
    "id\tpos\tcategory\tstereotype\tlemma\tlevel\n",
    "EN1\tn\tcds\tno\tbuffoon\tconservative\n",
);
#[test]
fn eval_prints_overall_accuracy() {
    let directory = tempdir().expect("temporary directory");
    fs::write(directory.path().join("hurtlex_EN.tsv"), EN_LEXICON).expect("write English fixture");
    let input_path = directory.path().join("eval.tsv");
    fs::write(
        &input_path,
        concat!(
            "language\tlabel\ttext\n",
            "EN\ttoxic\tYou are a buffoon\n",
            "EN\tclean\tThank you for your help\n",
        ),
    )
    .expect("write evaluation fixture");

    let output = Command::new(env!("CARGO_BIN_EXE_blasphem-train"))
        .args([
            "eval",
            "--data-dir",
            directory.path().to_str().expect("UTF-8 path"),
            "--input",
            input_path.to_str().expect("UTF-8 path"),
        ])
        .output()
        .expect("run blasphem");

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("UTF-8 output");
    assert!(stdout.contains("scope=overall n=2 tp=1 tn=1 fp=0 fn=0"));
    assert!(stdout.contains("accuracy=1.000"));
}

#[test]
fn eval_uses_policy_rules_without_a_lexical_match() {
    let directory = tempdir().expect("temporary directory");
    fs::write(directory.path().join("hurtlex_EN.tsv"), EN_LEXICON).expect("write English fixture");
    let input_path = directory.path().join("eval.tsv");
    fs::write(
        &input_path,
        concat!("language\tlabel\ttext\n", "EN\ttoxic\tI will kill you\n",),
    )
    .expect("write evaluation fixture");

    let output = Command::new(env!("CARGO_BIN_EXE_blasphem-train"))
        .args([
            "eval",
            "--data-dir",
            directory.path().to_str().expect("UTF-8 path"),
            "--input",
            input_path.to_str().expect("UTF-8 path"),
        ])
        .output()
        .expect("run blasphem");

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("UTF-8 output");
    assert!(stdout.contains("scope=overall n=1 tp=1 tn=0 fp=0 fn=0"));
}

#[test]
fn prepare_textdetox_publishes_all_files_after_deduplication() {
    let directory = tempdir().expect("temporary directory");
    let input_path = directory.path().join("source.tsv");
    let output_directory = directory.path().join("prepared");
    fs::write(
        &input_path,
        concat!(
            "source_id\tlanguage\ttoxic\ttext\n",
            "a\ten\t1\tYou are an idiot\n",
            "b\ten\t1\tyou are an IDIOT!\n",
            "c\ten\t0\tThank you\n",
        ),
    )
    .expect("write acquisition TSV");

    let rows = blasphem_train::parse_textdetox_rows(File::open(&input_path).expect("open source"))
        .expect("parse source");
    let prepared =
        blasphem_train::prepare_textdetox(&rows, &std::collections::BTreeSet::from(["EN".to_owned()]))
            .expect("prepare source");
    blasphem_train::publish_prepared_textdetox(&output_directory, &prepared).expect("publish source");
    for name in [
        "development.tsv",
        "validation.tsv",
        "test.tsv",
        "provenance.tsv",
    ] {
        assert!(output_directory.join(name).is_file(), "missing {name}");
    }
    assert_eq!(prepared.summary.duplicate_rows, 1);
}

#[test]
fn prepare_writes_publication_and_refuses_overwrite() {
    let directory = tempdir().expect("temporary directory");
    let (raw_root, lock, _) = write_prepare_fixture(directory.path());
    let output = directory.path().join("prepared");

    let first = blasphem_train_command(&[
        "prepare",
        "--source-lock",
        lock.to_str().expect("UTF-8 path"),
        "--raw-root",
        raw_root.to_str().expect("UTF-8 path"),
        "--output",
        output.to_str().expect("UTF-8 path"),
    ]);
    assert!(first.status.success(), "{:?}", first.stderr);
    assert!(output.join("manifest.json").is_file());

    let second = blasphem_train_command(&[
        "prepare",
        "--source-lock",
        lock.to_str().expect("UTF-8 path"),
        "--raw-root",
        raw_root.to_str().expect("UTF-8 path"),
        "--output",
        output.to_str().expect("UTF-8 path"),
    ]);
    assert!(!second.status.success());
}

#[test]
fn prepare_counts_audit_exclusions_in_the_manifest() {
    let directory = tempdir().expect("temporary directory");
    let (raw_root, lock, audit) = write_prepare_fixture(directory.path());
    let output = directory.path().join("prepared");

    let result = blasphem_train_command(&[
        "prepare",
        "--source-lock",
        lock.to_str().expect("UTF-8 path"),
        "--raw-root",
        raw_root.to_str().expect("UTF-8 path"),
        "--audit-exclusions",
        audit.to_str().expect("UTF-8 path"),
        "--output",
        output.to_str().expect("UTF-8 path"),
    ]);
    assert!(result.status.success(), "{:?}", result.stderr);

    let manifest: serde_json::Value =
        serde_json::from_reader(File::open(output.join("manifest.json")).expect("open manifest"))
            .expect("parse manifest");
    assert_eq!(manifest["exclusion_reason_counts"]["audit_only"], 1);
    assert_eq!(manifest["inclusion_status_counts"]["excluded"], 1);
}

#[test]
fn acquisition_accepts_a_valid_zero_row_source() {
    let mut client = FakeClient::new([
        revision_response("rev-a"),
        page_response(Some("rev-a"), 0, 0, 0),
        revision_response("rev-a"),
    ]);

    let acquired =
        acquire_textdetox(&mut client, &["en".to_owned()], None).expect("zero-row source");

    assert_eq!(acquired.revision, "rev-a");
    assert!(acquired.rows.is_empty());
    assert!(client.responses.is_empty());
}

#[test]
fn fetch_rejects_zero_max_rows_before_http() {
    let error = acquire_textdetox(&mut FakeClient::new([]), &["en".to_owned()], Some(0))
        .expect_err("zero rows");
    assert!(matches!(error, TextDetoxAcquisitionError::ZeroMaxRows));
}

#[test]
fn eval_rejects_allow_as_a_minimum_action() {
    let output = Command::new(env!("CARGO_BIN_EXE_blasphem-train"))
        .args([
            "eval",
            "--input",
            "missing.tsv",
            "--minimum-action",
            "allow",
        ])
        .output()
        .expect("run blasphem");

    assert!(!output.status.success());
    let stderr = String::from_utf8(output.stderr).expect("UTF-8 error");
    assert!(stderr.contains("invalid value 'allow'"));
}

#[test]
fn fetch_rejects_an_unsupported_source_language_before_http() {
    let error = textdetox_rows_url("pt", 0, 1).expect_err("unsupported source language");

    assert!(matches!(
        error,
        TextDetoxError::UnsupportedSourceLanguage(ref language) if language == "pt"
    ));
    assert_eq!(
        error.to_string(),
        "unsupported TextDetox source language: pt"
    );
}

#[test]
fn prepare_textdetox_refuses_an_existing_output_directory() {
    let directory = tempdir().expect("temporary directory");
    let output_directory = directory.path().join("prepared");
    fs::create_dir(&output_directory).expect("create existing output");
    let prepared = blasphem_train::PreparedTextDetox {
        development: Vec::new(),
        validation: Vec::new(),
        test: Vec::new(),
        provenance: Vec::new(),
        summary: blasphem_train::TextDetoxSummary::default(),
    };
    assert!(blasphem_train::publish_prepared_textdetox(&output_directory, &prepared).is_err());
    assert!(output_directory.is_dir());
}

#[test]
fn fetch_refuses_an_existing_output_before_http() {
    let directory = tempdir().expect("temporary directory");
    let output = directory.path().join("source.tsv");
    fs::write(&output, "existing").expect("write existing output");
    let mut client = FakeClient::new([]);

    let error = fetch_textdetox(&mut client, &["en".to_owned()], Some(1), &output)
        .expect_err("existing output");

    assert!(matches!(error, TextDetoxFetchError::ExistingOutput(path) if path == output));
    assert_eq!(
        fs::read_to_string(output).expect("existing output"),
        "existing"
    );
}

#[test]
fn acquisition_failure_leaves_no_final_or_staging_output() {
    let directory = tempdir().expect("temporary directory");
    let output = directory.path().join("source.tsv");
    let mut client = FakeClient::new([revision_response("rev-a"), page_response(None, 0, 1, 1)]);

    fetch_textdetox(&mut client, &["en".to_owned()], Some(1), &output)
        .expect_err("missing page revision");

    assert!(!output.exists());
    assert_eq!(
        fs::read_dir(directory.path())
            .expect("read directory")
            .count(),
        0
    );
}

#[test]
fn fetch_publishes_contiguous_rows_with_parser_source_ids() {
    let directory = tempdir().expect("temporary directory");
    let output = directory.path().join("source.tsv");
    let mut client = FakeClient::new([
        revision_response("rev-a"),
        page_response(Some("rev-a"), 0, 2, 2),
        revision_response("rev-a"),
    ]);

    let acquired =
        fetch_textdetox(&mut client, &["en".to_owned()], None, &output).expect("successful fetch");

    assert_eq!(acquired.revision, "rev-a");
    assert_eq!(
        acquired
            .rows
            .iter()
            .map(|row| row.source_id.as_str())
            .collect::<Vec<_>>(),
        ["textdetox@rev-a/en/000000", "textdetox@rev-a/en/000001"]
    );
    assert!(output.is_file());
    assert_eq!(
        fs::read_dir(directory.path())
            .expect("read directory")
            .count(),
        1
    );
}

#[test]
fn help_exposes_the_task_four_command_contract() {
    let help = Command::new(env!("CARGO_BIN_EXE_blasphem-train"))
        .arg("--help")
        .output()
        .expect("run help");
    let text = String::from_utf8(help.stdout).expect("UTF-8 help");
    for command in [
        "observe",
        "freeze-sources",
        "acquire",
        "prepare",
        "setup",
        "compile",
        "eval",
    ] {
        assert!(text.contains(command), "missing {command}");
    }
    assert!(!text.contains("fetch-textdetox"));
    assert!(!text.contains("prepare-textdetox"));

    let compile = Command::new(env!("CARGO_BIN_EXE_blasphem-train"))
        .args(["compile", "--help"])
        .output()
        .expect("run check help");
    let compile_help = String::from_utf8(compile.stdout).expect("UTF-8 help");
    for argument in [
        "--prepared-root",
        "--hurtlex-root",
        "--spanish-legacy",
        "--output",
    ] {
        assert!(compile_help.contains(argument), "missing {argument}");
    }
    assert!(!compile_help.contains("--development"));

    let eval = Command::new(env!("CARGO_BIN_EXE_blasphem-train"))
        .args(["eval", "--help"])
        .output()
        .expect("run eval help");
    assert!(
        String::from_utf8(eval.stdout)
            .expect("UTF-8 help")
            .contains("--minimum-action")
    );
}

struct FakeClient {
    responses: VecDeque<TextDetoxHttpResponse>,
}

impl FakeClient {
    fn new(responses: impl IntoIterator<Item = TextDetoxHttpResponse>) -> Self {
        Self {
            responses: responses.into_iter().collect(),
        }
    }
}

impl TextDetoxHttpClient for FakeClient {
    fn get(&mut self, _url: &str) -> Result<TextDetoxHttpResponse, TextDetoxTransportError> {
        self.responses
            .pop_front()
            .ok_or_else(|| TextDetoxTransportError::new("unexpected request"))
    }
}

fn revision_response(revision: &str) -> TextDetoxHttpResponse {
    TextDetoxHttpResponse {
        revision: None,
        body: format!(r#"{{"sha":"{revision}"}}"#).into_bytes(),
    }
}

fn page_response(
    revision: Option<&str>,
    row_index: usize,
    row_count: usize,
    total: usize,
) -> TextDetoxHttpResponse {
    let rows = (0..row_count)
        .map(|index| {
            format!(
                r#"{{"row_idx":{},"row":{{"text":"message","toxic":0}}}}"#,
                row_index + index
            )
        })
        .collect::<Vec<_>>()
        .join(",");
    TextDetoxHttpResponse {
        revision: revision.map(str::to_owned),
        body: format!(r#"{{"rows":[{rows}],"num_rows_total":{total}}}"#).into_bytes(),
    }
}

fn blasphem_train_command(arguments: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_blasphem-train"))
        .current_dir(repo_root())
        .args(arguments)
        .output()
        .expect("run blasphem-train")
}

fn repo_root() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn one_source_catalog(url: &str) -> String {
    format!(
        r#"{{"schema_version":"source-catalog-v1","sources":[{{"dataset":"ibrohim-budi","detector_language":"ID","source_file_id":"ibrohim-budi-re-dataset","requested_url":"{url}","revision_url":null,"requested_revision":"revision","archive_member":null,"file_path":"source.csv","license_id":"CC-BY-4.0","license_url":"https://example.test/license","citation":"Example citation","upstream_lineage":["https://example.test/source"],"lineage_status":"resolved"}}]}}"#
    )
}

fn one_source_lock(url: &str) -> String {
    format!(
        r#"{{"schema_version":"source-lock-v1","sources":[{{"dataset":"ibrohim-budi","detector_language":"ID","source_file_id":"ibrohim-budi-re-dataset","immutable_source_url":"{url}","archive_member":null,"revision":"revision","file_path":"source.csv","file_sha256":"0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef","license_id":"CC-BY-4.0","license_url":"https://example.test/license","citation":"Example citation","upstream_lineage":["https://example.test/source"],"lineage_status":"resolved"}}]}}"#
    )
}

fn one_textdetox_source_lock(url: &str, revision: &str) -> String {
    format!(
        r#"{{"schema_version":"source-lock-v1","sources":[{{"dataset":"textdetox","detector_language":"EN","source_file_id":"textdetox-en","immutable_source_url":"{url}","archive_member":null,"revision":"{revision}","file_path":"textdetox/en.tsv","file_sha256":"0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef","license_id":"CC-BY-4.0","license_url":"https://example.test/license","citation":"Example citation","upstream_lineage":["https://example.test/source"],"lineage_status":"resolved"}}]}}"#
    )
}

fn write_catalog_observation(path: &std::path::Path) {
    let catalog_path = repo_root().join("resources/datasets/source-catalog-v1.json");
    let catalog: blasphem_train::source_manifest::SourceCatalog =
        serde_json::from_reader(File::open(catalog_path).expect("open catalog"))
            .expect("parse catalog");
    let digest = blasphem_train::evidence::Sha256Digest::try_from(
        "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef".to_owned(),
    )
    .expect("digest");
    let observation = blasphem_train::source_manifest::SourceObservation {
        schema_version: "source-observation-v1".to_owned(),
        sources: catalog
            .sources
            .into_iter()
            .map(|source| blasphem_train::source_manifest::SourceRecord {
                dataset: source.dataset,
                detector_language: source.detector_language,
                source_file_id: source.source_file_id,
                immutable_source_url: source.requested_url,
                archive_member: source.archive_member,
                revision: source.requested_revision,
                file_path: source.file_path,
                file_sha256: digest.clone(),
                download_sha256: (source.dataset == blasphem_train::datasets::DatasetId::TextDetox)
                    .then(|| digest.clone()),
                acquired_at_unix_seconds: 1,
                license_id: source.license_id,
                license_url: source.license_url,
                citation: source.citation,
                upstream_lineage: source.upstream_lineage,
                lineage_status: source.lineage_status,
            })
            .collect(),
    };
    serde_json::to_writer(
        File::create(path).expect("create observation"),
        &observation,
    )
    .expect("write observation");
}

#[derive(Clone)]
struct FixtureRow {
    toxic: bool,
    text: String,
}

fn write_prepare_fixture(
    root: &std::path::Path,
) -> (std::path::PathBuf, std::path::PathBuf, std::path::PathBuf) {
    use blasphem::{Language, normalize_text};
    use blasphem_train::{
        datasets::{DatasetId, LineageStatus},
        evidence::Sha256Digest,
        source_manifest::{FrozenSource, FrozenSourceLock, SourceObservation, SourceRecord},
    };

    let raw_root = root.join("raw");
    fs::create_dir(&raw_root).expect("create raw root");
    let digest = Sha256Digest::try_from(
        "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef".to_owned(),
    )
    .expect("fixture digest");
    let mut records = Vec::new();
    let mut add_source = |dataset, language, id: &str, path: &str| {
        records.push(SourceRecord {
            dataset,
            detector_language: language,
            source_file_id: id.to_owned(),
            immutable_source_url: format!("https://example.test/{id}"),
            archive_member: None,
            revision: if dataset == DatasetId::TextDetox {
                Some(blasphem_train::TEXTDETOX_REVISION.to_owned())
            } else {
                Some("fixture-v1".to_owned())
            },
            file_path: path.to_owned(),
            file_sha256: digest.clone(),
            download_sha256: None,
            acquired_at_unix_seconds: 1,
            license_id: "CC-BY-4.0".to_owned(),
            license_url: "https://example.test/license".to_owned(),
            citation: "Fixture citation".to_owned(),
            upstream_lineage: vec!["https://example.test/lineage".to_owned()],
            lineage_status: LineageStatus::Resolved,
        });
    };

    let textdetox = [
        ("en", Language::En),
        ("zh", Language::Zh),
        ("ar", Language::Ar),
        ("fr", Language::Fr),
        ("hi", Language::Hi),
        ("ru", Language::Ru),
        ("ja", Language::Ja),
        ("de", Language::De),
        ("it", Language::It),
    ];
    let mut audit_source_id = None;
    for (code, language) in textdetox {
        let path = format!("textdetox/{code}.tsv");
        add_source(
            DatasetId::TextDetox,
            language,
            &format!("textdetox-{code}"),
            &path,
        );
        let rows = hash_fixture_rows(
            language,
            &format!("textdetox {code}"),
            false,
            if code == "en" { 2 } else { 1 },
        );
        let mut data = "source_id\tlanguage\ttoxic\ttext\n".to_owned();
        for (index, row) in rows.into_iter().enumerate() {
            let source_id = format!(
                "textdetox@{}/{code}/{index:06}",
                blasphem_train::TEXTDETOX_REVISION
            );
            if code == "en" && audit_source_id.is_none() {
                let normalized = normalize_text(&row.text);
                if blasphem_train::datasets::split_for_key(language, &normalized)
                    == blasphem_train::datasets::DatasetSplit::Development
                {
                    audit_source_id = Some(source_id.clone());
                }
            }
            writeln!(
                data,
                "{source_id}\t{code}\t{}\t{}",
                u8::from(row.toxic),
                row.text
            )
            .expect("write TextDetox row");
        }
        write_fixture_file(&raw_root, &path, &data);
    }

    let de_training_path = "datasets/germeval-2018-training/germeval2018.training.txt";
    let de_test_path = "datasets/germeval-2018-test/germeval2018.test.txt";
    add_source(
        DatasetId::GermEval2018,
        Language::De,
        "germeval-2018-training",
        de_training_path,
    );
    add_source(
        DatasetId::GermEval2018,
        Language::De,
        "germeval-2018-test",
        de_test_path,
    );
    write_fixture_file(
        &raw_root,
        de_training_path,
        "GermEval clean training\tOTHER\tOTHER\n",
    );
    write_fixture_file(
        &raw_root,
        de_test_path,
        "GermEval toxic test\tOFFENSE\tINSULT\n",
    );

    let id_path = "datasets/ibrohim-budi-re-dataset/re_dataset.csv";
    add_source(
        DatasetId::IbrohimBudi,
        Language::Ms,
        "ibrohim-budi-re-dataset",
        id_path,
    );
    let mut id_data = "Tweet,HS,Abusive,HS_Individual,HS_Group,HS_Religion,HS_Race,HS_Physical,HS_Gender,HS_Other,HS_Weak,HS_Moderate,HS_Strong\n".to_owned();
    for row in hash_fixture_rows(Language::Ms, "ibrohim", false, 1) {
        writeln!(
            id_data,
            "{},{} ,0,0,0,0,0,0,0,0,0,0,0",
            row.text,
            u8::from(row.toxic)
        )
        .expect("write Indonesian row");
    }
    id_data = id_data
        .replace(",0 ,0,", ",0,0,")
        .replace(",1 ,0,", ",1,0,");
    write_fixture_file(&raw_root, id_path, &id_data);

    let pt_path = "datasets/told-br-alpha/ToLD-BR_alpha.csv";
    add_source(DatasetId::ToldBr, Language::Pt, "told-br-alpha", pt_path);
    let mut pt_data = "text,homophobia_1,homophobia_2,homophobia_3,obscene_1,obscene_2,obscene_3,insult_1,insult_2,insult_3,racism_1,racism_2,racism_3,misogyny_1,misogyny_2,misogyny_3,xenophobia_1,xenophobia_2,xenophobia_3,obs_1,obs_2,obs_3\n".to_owned();
    for row in hash_fixture_rows(Language::Pt, "told", false, 1) {
        let mut labels = vec!["0.0"; 21];
        if row.toxic {
            labels[0] = "1.0";
            labels[1] = "1.0";
        }
        writeln!(pt_data, "{},{}", row.text, labels.join(",")).expect("write Portuguese row");
    }
    write_fixture_file(&raw_root, pt_path, &pt_data);

    let tr_train_path = "datasets/offenseval-tr-training/offenseval-tr-training-v1.tsv";
    let tr_test_path = "datasets/offenseval-tr-test/offenseval-tr-testset-v1.tsv";
    let tr_labels_path = "datasets/offenseval-tr-test-labels/offenseval-tr-labela-v1.tsv";
    add_source(
        DatasetId::OffensEvalTr,
        Language::Tr,
        "offenseval-tr-training",
        tr_train_path,
    );
    add_source(
        DatasetId::OffensEvalTr,
        Language::Tr,
        "offenseval-tr-test",
        tr_test_path,
    );
    add_source(
        DatasetId::OffensEvalTr,
        Language::Tr,
        "offenseval-tr-test-labels",
        tr_labels_path,
    );
    let mut tr_train = "id\ttweet\tsubtask_a\n".to_owned();
    for (index, row) in turkish_training_rows().into_iter().enumerate() {
        writeln!(
            tr_train,
            "train-{index}\t{}\t{}",
            row.text,
            if row.toxic { "OFF" } else { "NOT" }
        )
        .expect("write Turkish train row");
    }
    write_fixture_file(&raw_root, tr_train_path, &tr_train);
    let mut tr_test = "id\ttweet\n".to_owned();
    let mut tr_labels = String::new();
    for index in 0..600 {
        let toxic = index >= 300;
        writeln!(
            tr_test,
            "test-{index}\tturkish test {} {index}",
            if toxic { "toxic" } else { "clean" }
        )
        .expect("write Turkish test row");
        writeln!(
            tr_labels,
            "test-{index},{}",
            if toxic { "OFF" } else { "NOT" }
        )
        .expect("write Turkish label");
    }
    write_fixture_file(&raw_root, tr_test_path, &tr_test);
    write_fixture_file(&raw_root, tr_labels_path, &tr_labels);

    write_preserved_fixtures(
        &raw_root,
        &mut add_source,
        (
            DatasetId::ViHos,
            Language::Vi,
            "vihos",
            [
                "datasets/vihos-train/train.csv",
                "datasets/vihos-development/dev.csv",
                "datasets/vihos-test/test.csv",
            ],
        ),
    );
    write_preserved_fixtures(
        &raw_root,
        &mut add_source,
        (
            DatasetId::KMHas,
            Language::Ko,
            "kmhas",
            [
                "datasets/kmhas-train/kmhas_train.txt",
                "datasets/kmhas-validation/kmhas_valid.txt",
                "datasets/kmhas-test/kmhas_test.txt",
            ],
        ),
    );
    add_source(
        DatasetId::HurtLex,
        Language::Es,
        "hurtlex-es-1.2",
        "hurtlex/ES/1.2/hurtlex_ES.tsv",
    );

    let observation = SourceObservation {
        schema_version: "source-observation-v1".to_owned(),
        sources: records.clone(),
    };
    serde_json::to_writer(
        File::create(raw_root.join("source-observation-v1.json")).expect("create observation"),
        &observation,
    )
    .expect("write observation");
    let lock = FrozenSourceLock {
        schema_version: "source-lock-v1".to_owned(),
        sources: records
            .into_iter()
            .map(|source| FrozenSource {
                dataset: source.dataset,
                detector_language: source.detector_language,
                source_file_id: source.source_file_id,
                immutable_source_url: source.immutable_source_url,
                archive_member: source.archive_member,
                revision: source.revision,
                file_path: source.file_path,
                file_sha256: source.file_sha256,
                download_sha256: source.download_sha256,
                license_id: source.license_id,
                license_url: source.license_url,
                citation: source.citation,
                upstream_lineage: source.upstream_lineage,
                lineage_status: source.lineage_status,
            })
            .collect(),
    };
    let lock_path = root.join("source-lock.json");
    serde_json::to_writer(File::create(&lock_path).expect("create lock"), &lock)
        .expect("write lock");
    let audit_path = root.join("audit.tsv");
    fs::write(
        &audit_path,
        format!(
            "detector_language\tsource_id\treason\nEN\t{}\tfixture audit exclusion\n",
            audit_source_id.expect("English development row")
        ),
    )
    .expect("write audit");
    (raw_root, lock_path, audit_path)
}

fn hash_fixture_rows(
    language: blasphem::Language,
    prefix: &str,
    turkish: bool,
    development_quota: usize,
) -> Vec<FixtureRow> {
    let mut rows = Vec::new();
    for toxic in [false, true] {
        let mut development = 0;
        let mut validation = 0;
        for index in 0..100_000 {
            if development == development_quota && validation == 300 {
                break;
            }
            let text = format!(
                "{prefix} {} sample {index:06}",
                if toxic { "toxic" } else { "clean" }
            );
            let normalized = blasphem::normalize_text(&text);
            let remainder = blasphem_train::datasets::split_hash(language, &normalized) % 100;
            let is_development = if turkish {
                remainder <= 84
            } else {
                remainder <= 69
            };
            let is_validation = if turkish {
                remainder >= 85
            } else {
                (70..=84).contains(&remainder)
            };
            if is_development && development < development_quota
                || is_validation && validation < 300
            {
                rows.push(FixtureRow { toxic, text });
                if is_development {
                    development += 1;
                } else {
                    validation += 1;
                }
            }
        }
        assert_eq!(
            development, development_quota,
            "development quota for {prefix}"
        );
        assert_eq!(validation, 300, "validation quota for {prefix}");
    }
    if !turkish {
        for toxic in [false, true] {
            let mut test = 0;
            for index in 100_000..200_000 {
                if test == 300 {
                    break;
                }
                let text = format!(
                    "{prefix} {} test {index:06}",
                    if toxic { "toxic" } else { "clean" }
                );
                let normalized = blasphem::normalize_text(&text);
                if blasphem_train::datasets::split_for_key(language, &normalized)
                    == blasphem_train::datasets::DatasetSplit::Test
                {
                    rows.push(FixtureRow { toxic, text });
                    test += 1;
                }
            }
            assert_eq!(test, 300, "test quota for {prefix}");
        }
    }
    rows
}

fn turkish_training_rows() -> Vec<FixtureRow> {
    hash_fixture_rows(blasphem::Language::Tr, "turkish train", true, 1)
}

fn write_preserved_fixtures(
    raw_root: &std::path::Path,
    add_source: &mut impl FnMut(blasphem_train::datasets::DatasetId, blasphem::Language, &str, &str),
    (dataset, language, prefix, paths): (
        blasphem_train::datasets::DatasetId,
        blasphem::Language,
        &str,
        [&str; 3],
    ),
) {
    let source_ids = if dataset == blasphem_train::datasets::DatasetId::ViHos {
        ["vihos-train", "vihos-development", "vihos-test"]
    } else {
        ["kmhas-train", "kmhas-validation", "kmhas-test"]
    };
    add_source(dataset, language, source_ids[0], paths[0]);
    add_source(dataset, language, source_ids[1], paths[1]);
    add_source(dataset, language, source_ids[2], paths[2]);
    for (path, split, count) in [
        (paths[0], "train", 2),
        (paths[1], "validation", 600),
        (paths[2], "test", 600),
    ] {
        let mut content = if dataset == blasphem_train::datasets::DatasetId::ViHos {
            ",content,index_spans\n".to_owned()
        } else {
            "document\tlabel\n".to_owned()
        };
        for index in 0..count {
            let toxic = index >= count / 2;
            let text = format!(
                "{prefix} {split} {} {index:06}",
                if toxic { "toxic" } else { "clean" }
            );
            if dataset == blasphem_train::datasets::DatasetId::ViHos {
                writeln!(
                    content,
                    "{split}-{index},{text},{}",
                    if toxic { "[0]" } else { "[]" }
                )
                .expect("write Vietnamese row");
            } else {
                writeln!(content, "{text}\t{}", if toxic { "0" } else { "8" })
                    .expect("write Korean row");
            }
        }
        write_fixture_file(raw_root, path, &content);
    }
}

fn write_fixture_file(root: &std::path::Path, path: &str, contents: &str) {
    let path = root.join(path);
    fs::create_dir_all(path.parent().expect("fixture parent")).expect("create fixture parent");
    fs::write(path, contents).expect("write fixture file");
}
