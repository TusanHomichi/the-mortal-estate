#!/usr/bin/env python3
"""Prove a clean clone of this tree builds and tests with no private root.

The clean-room check asserts the private roots are absent and uncommittable.
That is the scanning half. This is the other half, and it is the one that
actually answers the question the genesis plan asked: **can this tree be handed
to someone who has none of the author's private files, and does it work?**

Method
------
1. Copy the *carried set* — every file git carries or would carry, the same set
   `tools/boundary_common.py` defines — into a scratch directory. Not
   `git clone`, deliberately: a clone would prove the last commit, and what
   gets reviewed and published is the working tree.
2. Assert the scratch copy contains **none of the roots `.gitignore` declares**.
   The list is read from `.gitignore` rather than restated here, so a root added
   to the ignore file is covered by this proof in the same edit.
3. Run the verification runner's own `portable` lane inside it, with
   `--allow-unavailable` — because a tree with no private denylist genuinely
   cannot prove the real denylist, and saying so is the honest answer.

Cost, owned rather than hoped for
---------------------------------
This is the second cold workspace build a complete run pays for, and on
2026-08-20 the pair of them filled a GitHub-hosted runner and killed the job
before a single step's log survived. So the build output is **this proof's own
property**, not a directory it leaves lying inside the copy:

* it goes to a target directory beside the copy, named here and printed;
* it is built with `verification.footprint.LEAN_BUILD_ENV` — no incremental
  state, line tables instead of full DWARF — because a disposable build has no
  next build to speed up and no debugger to attach;
* its **peak** size is sampled while the build runs and printed either way,
  because a run that cannot say what it cost is how the runner died;
* and it is removed in `finally`, before anything else, on success, on failure,
  and even under `--keep`. `--keep` exists to inspect the carried set; keeping
  several gigabytes of object files is not what anybody meant by it.

Measured on 2026-08-20 with the lean profile: see
[docs/agent-workflow.md](../docs/agent-workflow.md#the-disk-budget) for the
numbers this run is expected to produce and the budget they have to fit.
"""

from __future__ import annotations

import argparse
import os
import shutil
import subprocess
import sys
import tempfile
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent

sys.path.insert(0, str(ROOT / "tools"))

from verification import footprint  # noqa: E402  (tools/ is on the path above)


class CleanCloneError(RuntimeError):
    pass


def forbidden_roots(root: Path) -> tuple[str, ...]:
    """Every directory root `.gitignore` declares, read from the file itself.

    Restating them here would put a second list of private roots in a carried
    source file — which the clean-room check forbids for good reason, and which
    would drift the first time one was added.
    """
    entries: list[str] = []
    for line in (root / ".gitignore").read_text(encoding="utf-8").splitlines():
        entry = line.split("#", 1)[0].strip()
        if entry.endswith("/") and not entry.startswith("!"):
            entries.append(entry.rstrip("/").lstrip("/"))
    if not entries:
        raise CleanCloneError(".gitignore declares no ignored roots")
    return tuple(sorted(set(entries)))


def carried_paths(root: Path) -> list[str]:
    def listing(*arguments: str) -> list[str]:
        completed = subprocess.run(
            ["git", "-C", str(root), *arguments],
            capture_output=True,
            text=True,
            check=False,
        )
        if completed.returncode != 0:
            raise CleanCloneError(f"git {' '.join(arguments)} failed: {completed.stderr.strip()}")
        return [name for name in completed.stdout.split("\0") if name]

    names = set(listing("ls-files", "-z")) | set(
        listing("ls-files", "--others", "--exclude-standard", "-z")
    )
    return sorted(name for name in names if (root / name).is_file())


def populate(destination: Path, root: Path) -> int:
    names = carried_paths(root)
    for name in names:
        target = destination / name
        target.parent.mkdir(parents=True, exist_ok=True)
        shutil.copy2(root / name, target)
    return len(names)


def assert_clean(destination: Path, roots: tuple[str, ...]) -> None:
    present = [name for name in roots if (destination / name).exists()]
    if present:
        raise CleanCloneError(f"the copy is not clean; it carries {present}")


def prove(keep: bool, scope: str) -> int:
    scratch = Path(tempfile.mkdtemp(prefix="tme-clean-clone-"))
    workspace = scratch / "the-mortal-estate"
    # Beside the copy, never inside it. The copy is then exactly the carried
    # set for the whole run — there is no moment at which this proof's own
    # build output is sitting in the tree it is making claims about.
    build = scratch / "target"
    workspace.mkdir()
    build.mkdir()
    sampler = footprint.PeakFootprint(build)
    try:
        count = populate(workspace, ROOT)
        print(f"copied {count} carried files to {workspace}")
        assert_clean(workspace, forbidden_roots(ROOT))
        # A git work tree, because the boundary checks and the link check all
        # ask git what is carried. `git add -A` with no commit is enough: the
        # carried set is then exactly what was copied.
        for arguments in (
            ["init", "--quiet", "--initial-branch=main"],
            ["add", "-A"],
        ):
            subprocess.run(["git", "-C", str(workspace), *arguments], check=True)
        environment = footprint.lean_environment(os.environ)
        environment["CARGO_TARGET_DIR"] = str(build)
        print(f"build output: {build}")
        print(f"build profile: {footprint.lean_summary()}")
        print(f"before: {footprint.describe(build)}", flush=True)
        print(f"--- running the {scope} lane inside the clean copy ---", flush=True)
        with sampler:
            completed = subprocess.run(
                [
                    sys.executable,
                    "tools/run_verification.py",
                    "--scope",
                    scope,
                    "--allow-unavailable",
                ],
                cwd=str(workspace),
                env=environment,
                check=False,
            )
        if completed.returncode != 0:
            raise CleanCloneError(
                f"the clean copy's {scope} lane exited {completed.returncode}"
            )
    finally:
        # The build output goes first and unconditionally: it is the several
        # gigabytes, it is pure derivation, and a failure is exactly when a
        # machine can least afford to keep it.
        peak = footprint.mebibytes(sampler.peak_bytes)
        print(f"TME_CLEAN_CLONE_PEAK_MiB={peak} (build output at {build})")
        shutil.rmtree(build, ignore_errors=True)
        if keep:
            print(f"kept: {workspace} (build output removed)")
        else:
            shutil.rmtree(scratch, ignore_errors=True)
        print(f"after: {footprint.mebibytes(footprint.free_bytes(scratch.parent))} MiB free")
    print("TME_CLEAN_CLONE_OK — a clean copy of this tree builds, tests, and checks green")
    return 0


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(
        description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter
    )
    parser.add_argument(
        "--keep",
        action="store_true",
        help="leave the copied carried set in place; the build output is removed either way",
    )
    parser.add_argument(
        "--scope",
        default="portable",
        help="the lane to run inside the copy (default: portable)",
    )
    arguments = parser.parse_args(argv)
    try:
        return prove(arguments.keep, arguments.scope)
    except (CleanCloneError, OSError, subprocess.CalledProcessError) as error:
        print(f"clean-clone proof failed: {error}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
