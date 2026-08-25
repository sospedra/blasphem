use std::{fs, path::PathBuf};

use anyhow::{Context, Result};
use blasphem::{
    CandidateViewKind, Language, LanguageDetection, LanguageResolution, LanguageSelection,
    LanguageSource, MatchLevel, NudgeDetector, PolicyCategory, ReplyTarget,
};
#[cfg(feature = "language-detection")]
use blasphem::{LanguageDetector, resolve_language};
use clap::{Args, Parser, Subcommand, ValueEnum};

#[derive(Debug, Parser)]
#[command(
    name = "blasphem",
    about = "Experimental multilingual lexical toxicity detector"
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    Check(CheckArgs),
}

#[derive(Debug, Args)]
struct CheckArgs {
    #[arg(long)]
    language: LanguageSelection,
    #[arg(long)]
    text: String,
    #[arg(long, default_value = "data/raw-v1/hurtlex")]
    data_dir: PathBuf,
    #[arg(long, value_enum, default_value_t = ReplyTargetArg::Unknown)]
    reply_target: ReplyTargetArg,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum ReplyTargetArg {
    Unknown,
    Person,
    ProtectedGroup,
}

impl From<ReplyTargetArg> for ReplyTarget {
    fn from(value: ReplyTargetArg) -> Self {
        match value {
            ReplyTargetArg::Unknown => Self::Unknown,
            ReplyTargetArg::Person => Self::Person,
            ReplyTargetArg::ProtectedGroup => Self::ProtectedGroup,
        }
    }
}

fn main() -> Result<()> {
    match Cli::parse().command {
        Command::Check(arguments) => check(&arguments),
    }
}

fn check(arguments: &CheckArgs) -> Result<()> {
    let detection = match arguments.language {
        LanguageSelection::Explicit(language) => LanguageDetection {
            source: LanguageSource::Explicit,
            resolution: LanguageResolution::Known(language),
            reliable: true,
            score: None,
            feature_count: None,
        },
        LanguageSelection::Auto => automatic_detection(arguments)?,
    };

    let LanguageResolution::Known(language) = detection.resolution else {
        println!("ok=true score=0 threshold=50 should_nudge=false");
        println!(
            "language_mode=auto route=unknown detected_language=unknown reliable=false language_score=none evaluated=false"
        );
        return Ok(());
    };

    let path = arguments
        .data_dir
        .join(language.storage_code())
        .join("1.2")
        .join(format!("hurtlex_{}.tsv", language.storage_code()));
    let hurtlex = fs::read(&path).with_context(|| {
        format!(
            "cannot read required {} HurtLex data at {}",
            language.code(),
            path.display(),
        )
    })?;
    let detector = NudgeDetector::from_hurtlex_bytes(language, Some(&hurtlex))?;
    let mut result = detector.analyze(&arguments.text, arguments.reply_target.into());
    let nudge = result.nudge();
    println!(
        "ok={} score={} threshold={} should_nudge={}",
        !nudge.should_nudge, nudge.score, nudge.threshold, nudge.should_nudge,
    );
    print_routing_line(detection, language)?;
    println!("lexical_score={:.3}", result.lexical.score);
    match result.sparse_score {
        Some(score) => println!("sparse_score={score}"),
        None => println!("sparse_score=none"),
    }
    for (category, points) in [
        (PolicyCategory::Profanity, result.scores.profanity),
        (PolicyCategory::TargetedAbuse, result.scores.targeted_abuse),
        (
            PolicyCategory::IdentityAttack,
            result.scores.identity_attack,
        ),
        (
            PolicyCategory::ThreatLanguage,
            result.scores.threat_language,
        ),
        (
            PolicyCategory::SentimentSupport,
            result.scores.sentiment_support,
        ),
    ] {
        println!("category={category} points={points}");
    }
    println!("normalized={:?}", result.lexical.normalized_text);
    result.lexical.matches.sort_by(|left, right| {
        (
            left.raw_start,
            left.raw_end,
            view_order(left.view),
            &left.entry.language,
            &left.entry.id,
        )
            .cmp(&(
                right.raw_start,
                right.raw_end,
                view_order(right.view),
                &right.entry.language,
                &right.entry.id,
            ))
    });
    for found in result.lexical.matches {
        println!(
            "language={} lemma={:?} category={} level={} view={}",
            one_line(&found.entry.language),
            found.entry.lemma,
            one_line(&found.entry.category),
            display_level(found.entry.level),
            display_view(found.view),
        );
    }
    result.evidence.sort_by(|left, right| {
        (
            left.raw_start,
            left.raw_end,
            left.category.to_string(),
            left.rule_id.to_string(),
            &left.language,
            &left.matched_text,
        )
            .cmp(&(
                right.raw_start,
                right.raw_end,
                right.category.to_string(),
                right.rule_id.to_string(),
                &right.language,
                &right.matched_text,
            ))
    });
    for evidence in result.evidence {
        let language = one_line(evidence.language.as_deref().unwrap_or("none"));
        let span = match (evidence.raw_start, evidence.raw_end) {
            (Some(start), Some(end)) => format!("{start}:{end}"),
            _ => "none".to_owned(),
        };
        let view = evidence.candidate_view.map_or("none", display_view);
        let normalized_span = match (evidence.normalized_start, evidence.normalized_end) {
            (Some(start), Some(end)) => format!("{start}:{end}"),
            _ => "none".to_owned(),
        };
        println!(
            "rule={} category={} points={} language={} matched={:?} span={} view={} normalized_span={}",
            evidence.rule_id,
            evidence.category,
            evidence.points,
            language,
            evidence.matched_text,
            span,
            view,
            normalized_span,
        );
    }
    Ok(())
}

#[cfg(feature = "language-detection")]
fn automatic_detection(arguments: &CheckArgs) -> Result<LanguageDetection> {
    let detector = LanguageDetector::new()?;
    Ok(resolve_language(
        arguments.language,
        &arguments.text,
        &detector,
    ))
}

#[cfg(not(feature = "language-detection"))]
fn automatic_detection(_arguments: &CheckArgs) -> Result<LanguageDetection> {
    anyhow::bail!("AUTO requires the language-detection feature")
}

fn print_routing_line(detection: LanguageDetection, language: Language) -> Result<()> {
    match detection.source {
        LanguageSource::Explicit => println!(
            "language_mode=explicit route=known detected_language={} reliable=true language_score=none evaluated=true",
            language.code(),
        ),
        LanguageSource::Automatic => {
            let score = detection.score.context(
                "automatic language detection returned a known language without a score",
            )?;
            println!(
                "language_mode=auto route=known detected_language={} reliable=true language_score={score:.4} evaluated=true",
                language.code(),
            );
        }
    }
    Ok(())
}

fn one_line(value: &str) -> String {
    value.chars().flat_map(char::escape_debug).collect()
}

const fn view_order(view: CandidateViewKind) -> u8 {
    match view {
        CandidateViewKind::Normalized => 0,
        CandidateViewKind::Confusable => 1,
        CandidateViewKind::Evasion => 2,
    }
}

const fn display_view(view: CandidateViewKind) -> &'static str {
    match view {
        CandidateViewKind::Normalized => "normalized",
        CandidateViewKind::Confusable => "confusable",
        CandidateViewKind::Evasion => "evasion",
    }
}
fn display_level(level: MatchLevel) -> &'static str {
    match level {
        MatchLevel::Conservative => "conservative",
        MatchLevel::Inclusive => "inclusive",
    }
}
