"""What a build costs on disk, and the one lean profile a *proof* builds with.

The complete lane builds this workspace twice: once in the tree, and once
inside the copy `tools/run_clean_clone_proof.py` makes. On 2026-08-20 that pair
filled a GitHub-hosted runner and killed the job — `System.IO.IOException: No
space left on device`, on both attempts, twelve minutes in, with **no step log
written at all**. Nothing in the run had said what it was costing, so the only
evidence the failure left was that it had happened.

So two facts live here, and nowhere else.

**The lean profile.** `LEAN_BUILD_ENV` is what a proof build compiles with:
no incremental artifacts, and line tables instead of full DWARF. Both are
justified by what a proof is for. Incremental state exists to make the *next*
build fast; a proof build has no next build, so the state is pure cost. Full
debuginfo exists so a human can step through a binary in a debugger; a proof
reports a pass, a failure, and a backtrace, and line tables are enough to
resolve a backtrace to a file and a line.

It is an **environment** setting rather than a `[profile]` table in
`Cargo.toml` on purpose. The tracked profile is the *developer's* build — the
one that gets attached to a debugger on a machine with a disk — and quietly
degrading it for everyone to make a runner fit would be paying for CI with
everybody's tooling. The environment is where a caller says "this particular
build is disposable", and the two callers that say it are this repository's
clean-clone proof and its CI workflow. A local `--scope full` still builds the
way the developer's tree is configured to build.

**How a footprint is measured.** `directory_bytes` counts allocated blocks, not
apparent size, and counts a hard-linked inode once — a cargo target directory
has plenty of both, and either mistake would report a number the disk does not
recognise. `PeakFootprint` samples while a build runs, because cargo deletes
superseded artifacts as it goes: the size left behind at the end can be smaller
than the size the disk actually had to hold, and the number that decides
whether a runner survives is the peak.
"""

from __future__ import annotations

import os
import shutil
import threading
from pathlib import Path
from typing import Mapping

MEBIBYTE = 1024 * 1024

#: The disposable-build profile, as environment variables. Forced over whatever
#: the caller exported: a proof that builds with the ambient profile is not
#: measuring the thing it reports.
LEAN_BUILD_ENV: dict[str, str] = {
    "CARGO_INCREMENTAL": "0",
    "CARGO_PROFILE_DEV_DEBUG": "line-tables-only",
    "CARGO_PROFILE_TEST_DEBUG": "line-tables-only",
}


def lean_environment(environ: Mapping[str, str]) -> dict[str, str]:
    """`environ` with the disposable-build profile applied, caller value losing."""
    merged = dict(environ)
    merged.update(LEAN_BUILD_ENV)
    return merged


def lean_summary() -> str:
    """The profile as one line, so a log says what it built with."""
    return " ".join(f"{name}={value}" for name, value in sorted(LEAN_BUILD_ENV.items()))


def directory_bytes(path: Path | str) -> int:
    """Blocks allocated under `path`, each inode counted once.

    Matches what `du` reports rather than what `ls` does, and survives a
    directory that vanishes mid-walk — this runs against a tree cargo is
    actively rewriting.
    """
    total = 0
    seen: set[tuple[int, int]] = set()
    for root, _directories, filenames in os.walk(path, onerror=lambda _error: None):
        for name in filenames:
            try:
                info = os.lstat(os.path.join(root, name))
            except OSError:
                continue
            if info.st_nlink > 1:
                key = (info.st_dev, info.st_ino)
                if key in seen:
                    continue
                seen.add(key)
            total += info.st_blocks * 512
    return total


def mebibytes(value: int) -> int:
    """Bytes as whole MiB, rounded up. A footprint reported as 0 is a lie."""
    return (value + MEBIBYTE - 1) // MEBIBYTE


def free_bytes(path: Path | str) -> int:
    """Free space on the filesystem holding `path`, or its nearest live parent."""
    candidate = Path(path).resolve()
    while not candidate.exists() and candidate != candidate.parent:
        candidate = candidate.parent
    return shutil.disk_usage(candidate).free


def target_directory(environ: Mapping[str, str], root: Path) -> Path:
    """Where cargo will put build output for a run with this environment."""
    value = environ.get("CARGO_TARGET_DIR")
    return Path(value) if value else root / "target"


def describe(path: Path | str) -> str:
    """One line: what the directory costs, and what is left on its filesystem."""
    return (
        f"disk :: {path} = {mebibytes(directory_bytes(path))} MiB, "
        f"{mebibytes(free_bytes(path))} MiB free on its filesystem"
    )


class PeakFootprint:
    """The largest a directory got while something ran, sampled from a thread.

    Used as a context manager. The final sample is taken *after* the body
    exits, so a build that finishes between two ticks is still measured.
    """

    def __init__(self, path: Path | str, *, interval: float = 5.0) -> None:
        self._path = Path(path)
        self._interval = interval
        self._stop = threading.Event()
        self._lock = threading.Lock()
        self._peak = 0
        self._thread: threading.Thread | None = None

    @property
    def peak_bytes(self) -> int:
        with self._lock:
            return self._peak

    def sample(self) -> int:
        current = directory_bytes(self._path)
        with self._lock:
            self._peak = max(self._peak, current)
            return self._peak

    def _loop(self) -> None:
        while not self._stop.wait(self._interval):
            self.sample()

    def __enter__(self) -> "PeakFootprint":
        self.sample()
        self._thread = threading.Thread(target=self._loop, name="peak-footprint", daemon=True)
        self._thread.start()
        return self

    def __exit__(self, *_exception: object) -> bool:
        self._stop.set()
        if self._thread is not None:
            self._thread.join(timeout=self._interval * 2)
            self._thread = None
        self.sample()
        return False
