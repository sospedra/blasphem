"""Export an installed runtime with a reduced data selection."""

from __future__ import annotations

import argparse
import hashlib
import json
import shutil
import sys
import sysconfig
import tempfile
from importlib import metadata
from pathlib import Path

from . import BlasphemError, Judge, _manifest, _native, _normalize, _packs_directory, _read


def _arguments() -> argparse.Namespace:
    parser = argparse.ArgumentParser(prog="python -m blasphem")
    commands = parser.add_subparsers(dest="command", required=True)
    export = commands.add_parser("export", help="Copy the runtime and selected data into a deployment directory")
    export.add_argument("--locales", required=True)
    export.add_argument("--output", required=True, type=Path)
    export.add_argument("--no-detect", action="store_true")
    return parser.parse_args()


def _verified(directory: Path, name: str, expected: dict) -> bytes:
    data = _read(directory, name)
    if len(data) != expected["bytes"] or hashlib.sha256(data).hexdigest() != expected["sha256"]:
        raise BlasphemError("BLASPHEM_DIGEST_MISMATCH", f"Integrity mismatch for {name}")
    return data


def _export_data(target: Path, names: list[str]) -> None:
    source = _packs_directory(None)
    available = _manifest(source)
    missing = [name for name in names if name not in available]
    if missing:
        raise BlasphemError("BLASPHEM_LOCALE_MISSING", f"The installed data lacks {missing[0]}")
    target.mkdir()
    files = {name: available[name] for name in names}
    for name, expected in files.items():
        (target / name).write_bytes(_verified(source, name, expected))
    (target / "manifest.json").write_text(json.dumps({"formatVersion": 1, "files": files}) + "\n")
    notice = source / "NOTICE"
    shutil.copy2(notice, target / "NOTICE")


def _notice(name: str) -> Path:
    package_notice = Path(__file__).parent / name
    if package_notice.is_file():
        return package_notice
    distribution = metadata.distribution("blasphem")
    for entry in distribution.files or []:
        if entry.name == name:
            return Path(distribution.locate_file(entry))
    raise BlasphemError("BLASPHEM_ASSETS_REQUIRED", f"The installed runtime lacks its {name}")


def _export_runtime(target: Path) -> None:
    target.mkdir()
    source = Path(__file__).parent
    for path in source.glob("*.py"):
        shutil.copy2(path, target / path.name)
    extension = Path(_native.__file__)
    shutil.copy2(extension, target / extension.name)
    for name in ["NOTICE", "LICENSE"]:
        shutil.copy2(_notice(name), target / name)
    compatibility = {
        "python": sys.version,
        "platform": sysconfig.get_platform(),
        "extension": extension.name,
        "version": _version(),
    }
    (target / "deployment.json").write_text(json.dumps(compatibility, indent=2) + "\n")


def _version() -> str:
    deployed = Path(__file__).parent / "deployment.json"
    if deployed.is_file():
        return json.loads(deployed.read_text())["version"]
    return metadata.version("blasphem")


def export(arguments: argparse.Namespace) -> None:
    requested = "all" if arguments.locales == "all" else arguments.locales.split(",")
    codes = _normalize(requested)
    kinds = ["pack"] if arguments.no_detect else ["pack", "detect"]
    names = [f"{code}.{kind}" for code in codes for kind in kinds]
    output = arguments.output.resolve()
    if output.exists():
        raise FileExistsError(f"Output already exists: {output}")
    output.parent.mkdir(parents=True, exist_ok=True)
    staging = Path(tempfile.mkdtemp(prefix=".blasphem-export-", dir=output.parent))
    try:
        package = staging / "blasphem"
        _export_runtime(package)
        _export_data(package / "_data", names)
        with Judge(codes, assets=package / "_data", detect_language=not arguments.no_detect):
            staging.rename(output)
    finally:
        if staging.exists():
            shutil.rmtree(staging)
    print(f"status=exported locales={','.join(codes)} detect={not arguments.no_detect} to={output}")


if __name__ == "__main__":
    export(_arguments())
