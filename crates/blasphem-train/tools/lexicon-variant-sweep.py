#!/usr/bin/env python3
"""Generates Portuguese gender/number sibling candidates for a language's
built lexicon and corpus-checks each one with a word-boundary grep.

`crates/blasphem/src/text.rs` does NFC normalisation and confusable folding only, no
stemming, so masculine/feminine and singular/plural forms are independent
lemmas at the runtime matcher. This script exists because a form absent
from `{CODE}.senses.tsv` is a form the matcher will never catch, no matter
how well-attested its sibling is (Portuguese's `macaco`/`macaca`: the
masculine was correctly dropped for competing senses, but nothing ever
generated or corpus-checked the feminine, which turned out to be an
87.5%-toxic slur the masculine gives no hint of).

This script only proposes candidates and reports corpus evidence for a
human to read; it makes no decisions and writes nothing. Every candidate
still needs an individual read of its corpus hits before promotion, the
same discipline as the rest of this project's lexicon construction --
see `.superpowers/sdd/2026-09-03-clean-room-lexicon/task-3-PT-report.md`
for the worked example, including several rejected candidates (`feminista`,
the naive feminine of the coined mockery term `feministo`, is the ordinary
neutral word "feminist" and must not be added) that a human catches and a
mechanical rule cannot.

Morphology status: the feminine/plural rules below are Portuguese-specific,
verified by hand against every `-ão`-ending lemma in the Portuguese table
(the productive `-ão` -> `-ões` pattern, not the small closed set of
native irregulars like `cão`/`mão`/`alemão` that instead take `-ães`/
`-ãos` -- none of those appear in this lexicon's vocabulary, which is
built almost entirely on productive suffixation). They have NOT been
verified against Spanish or Italian. Spanish shares much of the basic o/a
gender pattern but has its own irregulars (`vez` -> `veces`); Italian
pluralises on a different axis entirely (`o` -> `i`, `a` -> `e`, no `-s`
suffix). Read the rules below and check them against real words in that
language's own table before trusting this script's output for anything
but Portuguese.

Usage:
    python3 lexicon-variant-sweep.py --storage-code PT
"""
from __future__ import annotations

import argparse
import csv
import re
import unicodedata
from pathlib import Path

TOOLS_DIR = Path(__file__).resolve().parent
REPO_ROOT = TOOLS_DIR.parent.parent.parent


def feminine_of(word: str) -> str | None:
    if " " in word or word.endswith("ão") or len(word) < 3:
        return None
    if word.endswith("o") and not word.endswith(("io", "uo")):
        return word[:-1] + "a"
    return None


def plural_of(word: str) -> str | None:
    if " " in word:
        return None
    if word.endswith("ão"):
        return word[:-2] + "ões"  # the productive pattern; see module docstring
    if word.endswith(("a", "e", "o", "á", "é", "ó", "í", "ú")):
        return word + "s"
    if word.endswith("m"):
        return word[:-1] + "ns"
    if word.endswith(("r", "z")):
        return word + "es"
    if word.endswith(("al", "el", "ol", "ul")):
        return word[:-1] + "is"
    if word.endswith("il"):
        return word[:-2] + "is"
    return None


VERB_INFINITIVE_SUFFIXES = ("ar", "er", "ir")


def corpus_counts(rows: list[dict], word: str) -> tuple[int, int]:
    pattern = re.compile(r"\b" + re.escape(word) + r"\b", re.IGNORECASE | re.UNICODE)
    toxic = clean = 0
    for row in rows:
        text = unicodedata.normalize("NFC", row["text"])
        if pattern.search(text):
            if row["label"] == "toxic":
                toxic += 1
            else:
                clean += 1
    return toxic, clean


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--storage-code", required=True, help='e.g. "PT"')
    parser.add_argument(
        "--data-root",
        default=REPO_ROOT / "resources" / "lexicon",
        type=Path,
        help="directory holding {CODE}.tsv",
    )
    parser.add_argument(
        "--corpus",
        default=None,
        type=Path,
        help="defaults to resources/corpus/{CODE}.tsv under the repo root",
    )
    args = parser.parse_args()
    code = args.storage_code.upper()
    corpus_path = args.corpus or (REPO_ROOT / "resources" / "corpus" / f"{code}.tsv")
    lexicon_path = args.data_root / f"{code}.tsv"

    rows = list(csv.DictReader(open(corpus_path), delimiter="\t"))
    lexicon = list(csv.DictReader(open(lexicon_path), delimiter="\t"))
    existing = {r["lemma"].strip().lower(): r for r in lexicon}

    candidates = []
    for lemma, row in existing.items():
        if " " in lemma or len(lemma) < 3:
            continue
        if any(lemma.endswith(suffix) for suffix in VERB_INFINITIVE_SUFFIXES):
            continue
        for kind, generator in (("feminine", feminine_of), ("plural", plural_of)):
            candidate = generator(lemma)
            if candidate and candidate.lower() not in existing:
                candidates.append((row["category"], lemma, kind, candidate))

    print(f"{len(candidates)} candidates generated from {len(existing)} lemmas")
    for category, base, kind, candidate in sorted(candidates):
        toxic, clean = corpus_counts(rows, candidate)
        if toxic + clean > 0:
            print(f"{category:4} {base:20} -> {kind:9} {candidate:20} tox={toxic:<3} cln={clean}")


if __name__ == "__main__":
    main()
