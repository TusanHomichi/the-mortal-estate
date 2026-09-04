"""Snapshot the carried source bytes for Workbench non-mutation proofs.

The boundary helper owns file selection: tracked and nonignored untracked
files, including Git's nested ignore rules and repository boundaries. The
clean-clone runner initializes a fresh Git index over its copied carried set
before running proofs, so it uses exactly the same policy.
"""

from __future__ import annotations

import hashlib
from pathlib import Path

from boundary_common import carried_files


def carried_tree(root: Path) -> dict[str, str]:
    """Hash regular carried files; never follow symlinks into external state.

    Inventory or read failures propagate: an unavailable snapshot cannot
    establish that the tree was unchanged.
    """
    return {
        name: hashlib.sha256((root / name).read_bytes()).hexdigest()
        for name in carried_files(root)
        if not (root / name).is_symlink()
    }
