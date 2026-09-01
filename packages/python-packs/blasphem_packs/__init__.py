"""The blasphem packs as an installable data package. `blasphem` reads them when no `assets` directory is given."""

from pathlib import Path


def directory() -> Path:
    """The directory that holds manifest.json and every .pack and .detect file."""
    return Path(__file__).resolve().parent
