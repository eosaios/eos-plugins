#!/usr/bin/env python3
"""Reproducible, generated source snapshot for independently buildable consumers."""
import argparse
import hashlib
import json
import shutil
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
p = argparse.ArgumentParser()
p.add_argument("--destination", type=Path, required=True)
p.add_argument("--check", action="store_true")
args = p.parse_args()
files = [ROOT / "Cargo.toml", ROOT / "Cargo.lock", ROOT / "LICENSE"]
for subdir in ("crates", "fixtures"):
    files += sorted(f for f in (ROOT / subdir).rglob("*") if f.is_file())
inventory = {str(f.relative_to(ROOT)): hashlib.sha256(f.read_bytes()).hexdigest() for f in files}
manifest = {"format": 1, "source": "eos-plugins", "version": "1.0.0", "files": inventory}
encoded = json.dumps(manifest, sort_keys=True, indent=2) + "\n"
dest = args.destination.resolve()
if args.check:
    if (dest / "source-lock.json").read_text() != encoded:
        raise SystemExit("plugin platform source lock is stale")
    for name, digest in inventory.items():
        if hashlib.sha256((dest / name).read_bytes()).hexdigest() != digest:
            raise SystemExit(f"stale generated plugin dependency: {name}")
else:
    dest.mkdir(parents=True, exist_ok=True)
    previous = json.loads((dest / "source-lock.json").read_text()) if (dest / "source-lock.json").exists() else {"files": {}}
    for old in previous["files"].keys() - inventory.keys():
        (dest / old).unlink()
    for f in files:
        target = dest / f.relative_to(ROOT)
        target.parent.mkdir(parents=True, exist_ok=True)
        shutil.copy2(f, target)
    (dest / "source-lock.json").write_text(encoded)
print(f"plugin platform snapshot verified: {dest}")
