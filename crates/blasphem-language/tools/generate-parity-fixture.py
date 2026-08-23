#!/usr/bin/env python3
"""Generate the frozen JSONL fixture with the pinned temporary C oracle."""

from __future__ import annotations

import json
import os
from pathlib import Path
import subprocess
import tempfile


TOOLS_DIR = Path(__file__).resolve().parent
CRATE_DIR = TOOLS_DIR.parent
UPSTREAM_ROOT = Path(
    os.environ.get("ELDC_UPSTREAM_ROOT", "/private/tmp/eldc-audit-20260902")
)
LANGUAGES = ("ar", "de", "en", "es", "fr", "hi", "it", "ja", "ko", "ms", "pt", "ru", "tr", "vi", "zh")


def upstream_unit_cases() -> list[dict[str, str]]:
    phrases = (
        "Bonjour le monde",
        "12345 !@#",
        "Was ist das? A ship. Todo bien en la costa.",
        "Was ist das?",
        "Hello world",
        "Bonjour",
        "Hola mundo",
        "Hola",
    )
    return [
        {"id": f"upstream-unit-{index:02}", "category": "upstream-unit", "input": text}
        for index, text in enumerate(phrases, 1)
    ]


def tatoeba_cases() -> list[dict[str, str]]:
    benchmark_dir = UPSTREAM_ROOT / "benchmark" / "text_files"
    language_path = benchmark_dir / "tatoeba_50_v3.languages.txt"
    text_path = benchmark_dir / "tatoeba_50_v3.txt"
    counts = {language: 0 for language in LANGUAGES}
    cases: list[dict[str, str]] = []

    with language_path.open(encoding="utf-8") as languages, text_path.open(encoding="utf-8") as texts:
        for language, text in zip(languages, texts):
            language = language.rstrip("\n")
            if language not in counts or counts[language] >= 5:
                continue
            counts[language] += 1
            cases.append(
                {
                    "id": f"tatoeba-{language}-{counts[language]:02}",
                    "category": f"tatoeba-{language}",
                    "input": text.rstrip("\n"),
                }
            )

    if any(count != 5 for count in counts.values()):
        raise RuntimeError(f"Tatoeba rows are incomplete: {counts}")
    return cases


def dense_unique_word_input() -> str:
    letters = [
        chr(codepoint)
        for codepoint in range(0x80, 0x800)
        if chr(codepoint).isalpha() and chr(codepoint).lower() == chr(codepoint)
    ][:200]
    cjk = [chr(0x4E00 + index) for index in range(200)]
    text = "".join(letter + ideograph for letter, ideograph in zip(letters, cjk))
    if len(text.encode("utf-8")) != 1_000:
        raise RuntimeError("the dense feature input must contain 1,000 UTF-8 bytes")
    return text


def edge_cases() -> list[dict[str, str]]:
    cases = (
        ("empty", ""),
        ("numeric", "1234567890 42 3.14159"),
        ("punctuation", "!@#$%^&*()_+-=[]{};:,.?/"),
        ("url", "https://example.com/path?q=hello&lang=en"),
        ("emoji-only", "😀🚀🧪❤️"),
        ("code-like", 'fn main() { println!("hello world"); }'),
        ("mixed-language", "Hello mundo bonjour العالم 你好"),
        ("ascii-apostrophe", "I don't know why Tom's here."),
        ("backtick", "This `code` isn't a sentence."),
        ("u2019-apostrophe", "L’avion n’est pas encore arrivé."),
        ("supplementary-cjk", "𠀀𠀁𠀂𠀃𠀄"),
        ("repeated-words", "hello hello hello hello hello hello"),
        ("bytes-999", "a" * 999),
        ("bytes-1000", "a" * 1_000),
        ("bytes-1001", "a" * 1_001),
        ("utf8-crosses-byte-1000", "a" * 999 + "é"),
        ("dense-unique-features", dense_unique_word_input()),
    )
    return [
        {"id": f"edge-{name}", "category": f"edge-{name}", "input": text}
        for name, text in cases
    ]


def run_oracle(inputs: list[str]) -> list[dict[str, object]]:
    if any("\n" in text or "\r" in text for text in inputs):
        raise ValueError("the line oracle does not accept newline characters")

    with tempfile.TemporaryDirectory(prefix="eldc-c-oracle-") as directory:
        oracle = Path(directory) / "eldc-c-oracle"
        subprocess.run([TOOLS_DIR / "build-c-oracle.sh", oracle], check=True)
        process = subprocess.run(
            [oracle],
            input="".join(f"{text}\n" for text in inputs),
            text=True,
            capture_output=True,
            check=True,
        )

    lines = process.stdout.splitlines()
    if len(lines) != len(inputs):
        raise RuntimeError(f"oracle returned {len(lines)} rows for {len(inputs)} inputs")
    return [parse_oracle_line(line) for line in lines]


def parse_oracle_line(line: str) -> dict[str, object]:
    fields = line.split("\t")
    score_count = int(fields[5])
    if len(fields) != 6 + score_count * 2:
        raise RuntimeError(f"invalid oracle row: {line}")
    ranked_scores = [
        {"language": fields[6 + index * 2], "score": float(fields[7 + index * 2])}
        for index in range(score_count)
    ]
    return {
        "language": None if fields[0] == "und" else fields[0],
        "reliable": fields[1] == "1",
        "feature_count": int(fields[2]),
        "top_score": float(fields[3]),
        "second_score": float(fields[4]),
        "ranked_scores": ranked_scores,
    }


def main() -> None:
    cases = [*upstream_unit_cases(), *tatoeba_cases(), *edge_cases()]
    results = run_oracle([case["input"] for case in cases])
    fixture_path = CRATE_DIR / "tests" / "fixtures" / "c-parity-v1.jsonl"
    with fixture_path.open("w", encoding="utf-8") as fixture:
        for case, result in zip(cases, results):
            json.dump({**case, **result}, fixture, ensure_ascii=False, separators=(",", ":"))
            fixture.write("\n")

    print(f"wrote {len(cases)} rows to {fixture_path}")


if __name__ == "__main__":
    main()
