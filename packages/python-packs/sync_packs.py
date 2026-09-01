"""Copies packages/packs/dist into blasphem_packs/ before `uv build` or `pip wheel`."""

from pathlib import Path
import shutil

here = Path(__file__).resolve().parent
source = here.parent / "packs" / "dist"
target = here / "blasphem_packs"
names = sorted(path.name for path in source.iterdir() if path.suffix in {".pack", ".detect"} or path.name == "manifest.json")
if not names:
    raise SystemExit(f"{source} is empty; run: pnpm --filter @blasphem/packs run build")
for name in names:
    shutil.copyfile(source / name, target / name)
total = sum((target / name).stat().st_size for name in names)
print(f"status=synced files={len(names)} mb={total / 1048576:.2f}")
