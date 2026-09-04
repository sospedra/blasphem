"""Export the canonical resources/packs artifacts before building a Python package."""

import hashlib
import json
import os
from pathlib import Path
import sys
import tempfile

here = Path(__file__).resolve().parent
source = Path(sys.argv[1]).resolve() if len(sys.argv) > 1 else here.parent.parent / "resources" / "packs"
target = here / "blasphem_packs"


def read_pack(name, record):
    parts = name.split(".")
    if len(parts) != 2 or parts[1] not in {"pack", "detect"}:
        raise ValueError(f"Invalid pack name: {name}")
    if len(parts[0]) != 2 or not all("a" <= letter <= "z" for letter in parts[0]):
        raise ValueError(f"Invalid locale: {name}")
    path = source / name
    if path.is_symlink() or not path.is_file():
        raise ValueError(f"Expected a regular file: {path}")
    data = path.read_bytes()
    if type(record.get("bytes")) is not int or record["bytes"] <= 0:
        raise ValueError(f"Invalid size: {name}")
    if len(data) != record["bytes"] or hashlib.sha256(data).hexdigest() != record.get("sha256"):
        raise ValueError(f"Pack integrity mismatch: {path}")
    return data


def load_packs():
    manifest_bytes = (source / "manifest.json").read_bytes()
    manifest = json.loads(manifest_bytes)
    if manifest.get("formatVersion") != 1 or not isinstance(manifest.get("files"), dict):
        raise ValueError(f"Invalid packs manifest: {source}")
    if not manifest["files"]:
        raise ValueError(f"Empty packs manifest: {source}")
    files = {name: read_pack(name, record) for name, record in sorted(manifest["files"].items())}
    unlisted = {path.name for path in source.iterdir() if path.suffix in {".pack", ".detect"}} - files.keys()
    if unlisted:
        raise ValueError(f"Unlisted packs: {sorted(unlisted)}")
    return {**files, "NOTICE": (here.parent.parent / "NOTICE").read_bytes(), "manifest.json": manifest_bytes}


def export_packs(files):
    target.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory(prefix=".packs-export-", dir=here) as temporary:
        staged = Path(temporary)
        for name, data in files.items():
            destination = staged / name
            destination.write_bytes(data)
            if destination.read_bytes() != data:
                raise ValueError(f"Export integrity mismatch: {destination}")
        for name in files:
            os.replace(staged / name, target / name)
    for path in target.iterdir():
        if path.suffix in {".pack", ".detect"} and path.name not in files:
            path.unlink()


files = load_packs()
export_packs(files)
total = sum(map(len, files.values()))
print(f"status=synced files={len(files)} mb={total / 1048576:.2f} source={source}")
