"""Bind an explicitly selected private study packet into an immutable release."""
import json
import shutil
from pathlib import Path
from common import REPO, digest


def copy_artwork(source: Path, destination: Path):
    source = source.resolve(strict=True)
    if not source.is_dir() or source.is_relative_to(REPO):
        raise ValueError("presentation assets must be an external directory")
    manifest = source / "feel-manifest.json"
    receipt = json.loads((REPO / "web/src/play/studyReceipt.json").read_text())
    if digest(manifest) != receipt["asset_manifest_sha256"]:
        raise ValueError("presentation packet differs from this browser's receipt")
    files = {"feel-manifest.json": digest(manifest)}

    def visit(value):
        if isinstance(value, dict):
            if "file" in value and "sha256" in value:
                name, expected = value["file"], value["sha256"]
                relative = Path(name)
                if relative.is_absolute() or ".." in relative.parts or relative.as_posix() != name:
                    raise ValueError("presentation asset name must be a normalized relative path")
                path = source / name
                if not path.resolve(strict=True).is_relative_to(source) or path.is_symlink() or not path.is_file():
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
    visit(json.loads(manifest.read_text()))
    destination.mkdir(parents=True)
    for name, expected in files.items():
        target = destination / name
        target.parent.mkdir(parents=True, exist_ok=True)
        shutil.copyfile(source / name, target)
        if digest(target) != expected:
            raise RuntimeError("presentation asset changed while copying")
    return files
