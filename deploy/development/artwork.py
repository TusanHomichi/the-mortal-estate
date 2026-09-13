"""Bind an explicitly selected private study packet into an immutable release."""
import json
import shutil
from pathlib import Path
from common import REPO, digest

# The browser fetches every presentation model from these source-owned receipts;
# both sets are validated together so neither can be quietly skipped.
RECEIPTS = ("web/src/play/settlementReceipt.json", "web/src/play/dungeon/receipt.json")


def presentation_files(source: Path):
    """Read-only: the pinned GLBs a packet must carry, as name -> sha256."""
    try:
        source = source.resolve(strict=True)
    except OSError as error:
        raise ValueError("presentation assets must be an external directory") from error
    if not source.is_dir() or source.is_relative_to(REPO):
        raise ValueError("presentation assets must be an external directory")
    files = {}

    def visit(value):
        if isinstance(value, dict):
            if "file" in value and "sha256" in value:
                name, expected = value["file"], value["sha256"]
                relative = Path(name)
                if relative.suffix != ".glb":
                    raise ValueError("presentation asset has the wrong format for its receipt")
                if relative.is_absolute() or ".." in relative.parts or relative.as_posix() != name:
                    raise ValueError("presentation asset name must be a normalized relative path")
                path = source / name
                if not path.resolve().is_relative_to(source) or path.is_symlink() or not path.is_file():
                    raise ValueError("presentation asset escapes the packet or is not a regular file")
                if name in files and files[name] != expected:
                    raise ValueError("presentation asset has conflicting identities")
                if digest(path) != expected:
                    raise ValueError("presentation asset digest mismatch")
                files[name] = expected
            for child in value.values():
                visit(child)
        elif isinstance(value, list):
            for child in value:
                visit(child)

    for receipt in RECEIPTS:
        visit(json.loads((REPO / receipt).read_text()))
    if not files:
        raise ValueError("presentation receipts pin no assets")
    return files


def copy_artwork(source: Path, destination: Path):
    files = presentation_files(source)
    source = source.resolve()
    destination.mkdir(parents=True)
    for name, expected in files.items():
        target = destination / name
        target.parent.mkdir(parents=True, exist_ok=True)
        shutil.copyfile(source / name, target)
        if digest(target) != expected:
            raise RuntimeError("presentation asset changed while copying")
    return files
