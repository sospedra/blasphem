#!/usr/bin/env python3
"""Generates Portuguese gender/number sibling candidates for a language's
*dropped* lemmas (`{CODE}.drops.txt`) and corpus-checks each one.

`lexicon-variant-sweep.py` only re-inflects lemmas already KEPT, so by
construction it can never find a case like Portuguese's `macaco`/`macaca`:
`macaco` was dropped for having real competing senses (a favela name, an
actual pet-monkey wish), so nothing ever generated its feminine -- which
turned out to be an 87.5%-toxic slur with none of the masculine's
ambiguity. A base form can be genuinely ambiguous while its gender/number
sibling is not; the only way to find that is to sweep the drop list too.

Read every candidate this reports before deciding anything -- most will be
real words that simply confirm the base form's own exclusion reasoning
(Portuguese's own sweep found `negros`, `cabras`, `nega`, `velha` all show
the same neutral/address-term pattern as their already-excluded bases, no
live miss). Occasionally one won't (Portuguese's own sweep separately
found `porco`/`porca`, a real gap where *neither* gendered form had ever
been considered, not a case of gendered ambiguity at all). Both outcomes
are useful; this script produces the list to read, not the verdict.

See `lexicon-variant-sweep.py`'s docstring for the morphology rules' scope
and limits (Portuguese-verified only, not Spanish or Italian).

Usage:
    python3 lexicon-drops-sweep.py --storage-code PT
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
        return word[:-2] + "ões"
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
        "--data-root", default=REPO_ROOT / "lexicon", type=Path
    )
    parser.add_argument("--corpus", default=None, type=Path)
    parser.add_argument(
        "--min-toxic",
        type=int,
        default=1,
        help="only print candidates with at least this many toxic hits",
    )
    args = parser.parse_args()
    code = args.storage_code.upper()
    corpus_path = args.corpus or (REPO_ROOT / "corpus" / f"{code}.tsv")
    senses_path = args.data_root / f"{code}.senses.tsv"
    drops_path = args.data_root / f"{code}.drops.txt"

    rows = list(csv.DictReader(open(corpus_path), delimiter="\t"))
    kept = {
        r["lemma"].strip().lower()
        for r in csv.DictReader(open(senses_path), delimiter="\t")
    }
    drops = [
        line.strip().lower() for line in open(drops_path) if line.strip()
    ]
    drop_set = set(drops)

    candidates = []
    for lemma in drops:
        if " " in lemma or len(lemma) < 3:
            continue
        for kind, generator in (("feminine", feminine_of), ("plural", plural_of)):
            candidate = generator(lemma)
            if candidate and candidate not in kept and candidate not in drop_set:
                candidates.append((lemma, kind, candidate))

    print(f"{len(drops)} dropped lemmas; {len(candidates)} candidates generated")
    flagged = 0
    for base, kind, candidate in candidates:
        toxic, clean = corpus_counts(rows, candidate)
        if toxic >= args.min_toxic:
            flagged += 1
            base_toxic, base_clean = corpus_counts(rows, base)
            print(
                f"{base:16} -> {kind:9} {candidate:18} "
                f"base(tox={base_toxic:<3} cln={base_clean:<3}) "
                f"candidate(tox={toxic:<3} cln={clean})"
            )
    print(f"{flagged} candidates with >= {args.min_toxic} toxic hit(s)")


if __name__ == "__main__":
    main()
