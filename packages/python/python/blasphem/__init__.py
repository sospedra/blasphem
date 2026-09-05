"""Multilingual pre-send toxicity nudge.

Blasphem hashes word and character n-grams into sparse feature vectors.
A linear classifier trained offline scores them with 16-bit weights.
Lexicons and context rules contribute to the verdict.
Detection runs locally without neural networks or cloud inference.

    import blasphem

    blasphem.init(["en", "es"], grawlix=True)
    blasphem.judge("you are a stupid loser")
    # Judgement(safe=False, score=0.95, locale="en", grawlix="you are a @#$%&! @#$%&")

``init`` loads the locales once and installs the module judge. ``judge`` is
synchronous and never raises: before ``init`` and after ``close`` it returns
the fail-open verdict. ``Judge`` builds an independent judge when one per
module is not enough. Packs come from ``assets`` (a directory) or from the
installed ``blasphem_packs`` package.
"""

from __future__ import annotations

import json
import re
import threading
from dataclasses import dataclass
from pathlib import Path
from typing import Iterable

from . import _native
from ._locales import LOCALES

__all__ = ["BlasphemError", "Judge", "Judgement", "close", "init", "judge", "ready"]

_CODES = frozenset(
    {
        "BLASPHEM_LOCALES_EMPTY",
        "BLASPHEM_LOCALE_UNSUPPORTED",
        "BLASPHEM_LOCALE_MISSING",
        "BLASPHEM_ASSETS_REQUIRED",
        "BLASPHEM_FETCH_FAILED",
        "BLASPHEM_DIGEST_MISMATCH",
        "BLASPHEM_FORMAT_VERSION",
        "BLASPHEM_PACK_INVALID",
        "BLASPHEM_CLOSED",
    }
)
_MANIFEST_FORMAT_VERSION = 1
_HEX64 = re.compile(r"^[0-9a-f]{64}$")
_CANONICAL = {code: code for code, _ in LOCALES} | {alias: code for code, aliases in LOCALES for alias in aliases}
_ORDER = {code: index for index, (code, _) in enumerate(LOCALES)}


class BlasphemError(Exception):
    """Every failure of ``init`` and ``Judge``. ``code`` is the contract code."""

    def __init__(self, code: str, message: str) -> None:
        super().__init__(f"{code}: {message}")
        self.code = code
        self.message = message

    @classmethod
    def from_native(cls, error: BaseException) -> "BlasphemError":
        text = str(error)
        code, separator, detail = text.partition(": ")
        if separator and code in _CODES:
            return cls(code, detail)
        return cls("BLASPHEM_PACK_INVALID", text)


@dataclass(frozen=True)
class Judgement:
    """One verdict for one message."""

    safe: bool
    """True when no nudge is due. Unroutable text is safe; the nudge fails open."""
    score: float
    """Ordinal risk from 0 through 1. Not a probability."""
    locale: str | None
    """The locale that produced the score, or None."""
    grawlix: str | None
    """Masked text for unsafe verdicts when requested, otherwise None."""


def _fail_open() -> Judgement:
    return Judgement(True, 0.0, None, None)


def _normalize(locales: Iterable[str] | str | None) -> list[str]:
    if locales == "all":
        return [code for code, _ in LOCALES]
    if isinstance(locales, str):
        raise BlasphemError("BLASPHEM_LOCALE_UNSUPPORTED", 'locales must be an array or "all"')
    requested = list(locales or [])
    if not requested:
        raise BlasphemError("BLASPHEM_LOCALES_EMPTY", 'pass at least one locale, such as ["en"]')
    codes: dict[str, None] = {}
    for raw in requested:
        code = _CANONICAL.get(str(raw).strip().lower())
        if code is None:
            raise BlasphemError("BLASPHEM_LOCALE_UNSUPPORTED", f"unsupported locale {raw!r}")
        codes[code] = None
    return sorted(codes, key=_ORDER.__getitem__)


def _packs_directory(assets: str | Path | None) -> Path:
    if str(assets) in {"remote", "jsdelivr"} or "://" in str(assets):
        raise BlasphemError("BLASPHEM_ASSETS_REQUIRED", "Python supports only bundled data or a local directory")
    if assets not in {None, "bundled"} and str(assets).strip():
        return Path(assets)
    exported = Path(__file__).parent / "_data"
    if exported.is_dir():
        return exported
    try:
        import blasphem_packs  # type: ignore[import-not-found]
    except ImportError as error:
        raise BlasphemError("BLASPHEM_ASSETS_REQUIRED", f"The internal data dependency is unavailable: {error}") from None
    return Path(blasphem_packs.directory())


def _read(directory: Path, name: str) -> bytes:
    try:
        return (directory / name).read_bytes()
    except OSError as error:
        raise BlasphemError("BLASPHEM_FETCH_FAILED", f"{name}: {error}") from None


def _manifest(directory: Path) -> dict[str, dict[str, object]]:
    try:
        parsed = json.loads(_read(directory, "manifest.json"))
    except ValueError as error:
        raise BlasphemError("BLASPHEM_PACK_INVALID", f"manifest.json is not JSON: {error}") from None
    if not isinstance(parsed, dict) or parsed.get("formatVersion") != _MANIFEST_FORMAT_VERSION:
        found = parsed.get("formatVersion") if isinstance(parsed, dict) else None
        raise BlasphemError("BLASPHEM_FORMAT_VERSION", f"manifest.json has format version {found}, this build accepts {_MANIFEST_FORMAT_VERSION}")
    files = parsed.get("files")
    if not isinstance(files, dict):
        raise BlasphemError("BLASPHEM_PACK_INVALID", "manifest.json lacks a files map")
    for name, record in files.items():
        if not re.fullmatch(r"[a-z]{2,3}\.(pack|detect)", name):
            raise BlasphemError("BLASPHEM_PACK_INVALID", f"manifest.json has an invalid filename {name!r}")
        if not isinstance(record, dict) or not isinstance(record.get("sha256"), str) or not _HEX64.match(record["sha256"]):
            raise BlasphemError("BLASPHEM_PACK_INVALID", f"manifest.json entry {name!r} needs a 64-character sha256")
        if type(record.get("bytes")) is not int or record["bytes"] <= 0:
            raise BlasphemError("BLASPHEM_PACK_INVALID", f"manifest.json entry {name!r} needs a positive byte length")
    return files


def _sized_read(directory: Path, name: str, files: dict) -> bytes:
    data = _read(directory, name)
    if len(data) != files[name]["bytes"]:
        raise BlasphemError("BLASPHEM_DIGEST_MISMATCH", f"{name} has the wrong length")
    return data


def _entries(directory: Path, codes: list[str], detect_language: bool) -> list[tuple]:
    files = _manifest(directory)

    def digest(name: str, code: str) -> str:
        record = files.get(name)
        if record is None:
            raise BlasphemError("BLASPHEM_LOCALE_MISSING", f"manifest.json lists no {name}; the packs do not include {code}")
        return str(record["sha256"])

    entries = []
    for code in codes:
        pack_name = f"{code}.pack"
        pack_sha = digest(pack_name, code)
        detect_bytes = detect_sha = None
        if detect_language:
            detect_name = f"{code}.detect"
            detect_sha = digest(detect_name, code)
            detect_bytes = _sized_read(directory, detect_name, files)
        entries.append((code, _sized_read(directory, pack_name, files), pack_sha, detect_bytes, detect_sha))
    return entries


class Judge:
    """One judge over a fixed set of locales. Safe to share between threads."""

    def __init__(
        self,
        locales: Iterable[str] | str,
        *,
        assets: str | Path | None = None,
        detect_language: bool = True,
        grawlix: bool = False,
    ) -> None:
        codes = _normalize(locales)
        directory = _packs_directory(assets)
        entries = _entries(directory, codes, detect_language)
        try:
            self._engine = _native.Engine(entries, detect_language, grawlix)
        except ValueError as error:
            raise BlasphemError.from_native(error) from None
        self._open = True

    @property
    def locales(self) -> list[str]:
        """The loaded locales, in registry order."""
        return list(self._engine.locales)

    def judge(self, text: str) -> Judgement:
        """Scores one message. Raises ``BlasphemError`` with code ``BLASPHEM_CLOSED`` after ``close``."""
        try:
            safe, score, locale, grawlix = self._engine.judge(text)
        except ValueError as error:
            raise BlasphemError.from_native(error) from None
        return Judgement(safe, score, locale, grawlix)

    def close(self) -> None:
        """Releases the packs."""
        self._open = False
        self._engine.close()

    def __enter__(self) -> "Judge":
        return self

    def __exit__(self, *exc: object) -> None:
        self.close()


_lock = threading.Lock()
_current: Judge | None = None
_current_key: str | None = None


def _key(locales: Iterable[str], assets: str | Path | None, detect_language: bool, grawlix: bool) -> str:
    return json.dumps(
        {"locales": sorted(str(code).strip().lower() for code in (locales or [])), "assets": str(assets) if assets else None, "detect": detect_language, "grawlix": grawlix},
        sort_keys=True,
    )


def init(
    locales: Iterable[str] | str,
    *,
    assets: str | Path | None = None,
    detect_language: bool = True,
    grawlix: bool = False,
) -> None:
    """Loads the locales and installs the module judge.

    The same options again reuse the judge. Different options build a new one
    and retire the old one after, so ``judge`` has no gap. A failed ``init``
    raises ``BlasphemError`` and keeps the previous judge.
    """
    global _current, _current_key
    codes = _normalize(locales)
    key = _key(codes, assets, detect_language, grawlix)
    with _lock:
        if _current is not None and _current_key == key:
            return
        replacement = Judge(codes, assets=assets, detect_language=detect_language, grawlix=grawlix)
        previous, _current, _current_key = _current, replacement, key
    if previous is not None:
        previous.close()


def judge(text: str) -> Judgement:
    """Scores one message. Before ``init`` and after ``close`` it fails open. Never raises."""
    current = _current
    if current is None:
        return _fail_open()
    try:
        return current.judge(text)
    except BlasphemError:
        return _fail_open()


def ready() -> bool:
    """True between ``init`` and ``close``."""
    return _current is not None


def close() -> None:
    """Releases the module judge. ``judge`` fails open until the next ``init``."""
    global _current, _current_key
    with _lock:
        previous, _current, _current_key = _current, None, None
    if previous is not None:
        previous.close()
