"""Integrity admission shared by immutable native presentation proofs."""
import hashlib
import json
from pathlib import Path

from live_server_harness import REPOSITORY_ROOT


def checked_release(path: Path) -> Path:
    release = path.resolve(strict=True)
    if release.is_relative_to(REPOSITORY_ROOT):
        raise ValueError("release must be outside the checkout")
    receipt = json.loads((release / "release.json").read_text())
    actual = {str(p.relative_to(release)): hashlib.sha256(p.read_bytes()).hexdigest()
              for p in release.rglob("*") if p.is_file() and p != release / "release.json"}
    if any(p.is_symlink() for p in release.rglob("*")) or actual != receipt["files"]:
        raise ValueError("release differs from its integrity receipt")
    for name, digest in actual.items():
        if name.startswith("content/") and hashlib.sha256((REPOSITORY_ROOT / name).read_bytes()).hexdigest() != digest:
            raise ValueError("proof world content differs from the release")
    return release
