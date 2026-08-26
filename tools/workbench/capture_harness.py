"""Taking a new capture: the one place in the Workbench that runs a program.

Everything else in this package reads files. This module runs the shipped
client, because a capture is a picture of what the client draws and there is no
honest way to produce one without the client drawing it. Spec open decision 7
warned against exactly the alternative — pretending an expensive lane is cheap —
so the cost is here, named, bounded, and off the ordinary selection path.

**What it costs.** Seconds, not minutes: no server, no database, no credentials,
no account. The client mounts the world view alone, presents one recorded
authoritative frame, and captures it. The recorded frame is a real server frame,
produced by `tools/run_fixture_land_capture.py`, which is the expensive lane and
stays the accuracy reference rather than the interactive one.

**What it requires, and never guesses at.** The pinned client binary, named by
environment, and a virtual display, because the client's headless display driver
produces no viewport image at all. Neither has a fallback: a capture taken by
something other than the shipped presenter would not be a capture of the shipped
presenter, and a blank picture with a confident sidecar is worse than a refusal.
"""

from __future__ import annotations

import os
import shutil
import subprocess
from pathlib import Path

from .capture import CAPTURES_DIR, Capture, CaptureUnavailable, load, next_capture_id

#: The client harness that draws one recorded frame and captures it.
HARNESS_SCRIPT = "res://tests/capture_fixture_frame.gd"

#: The recorded authoritative frame the harness replays. Produced by the live
#: route; never synthesised.
FRAME_FIXTURE = "tests/fixtures/capture/fixture_land_frame.json"

SUCCESS_SENTINEL = "TME_CAPTURE_OK"

#: The pinned client binary, named by environment because it is a machine fact
#: rather than a repository one.
GODOT_VARIABLE = "TME_GODOT"

#: A virtual display at a fixed geometry, so two captures of one frame differ
#: only where the renderer differs.
XVFB = "xvfb-run"
XVFB_SCREEN = "1280x1024x24"
CAPTURE_WINDOW = (1024, 768)
CAPTURE_TIMEOUT_SECONDS = 180.0


def harness_command(root: Path, godot: str) -> list[str]:
    """The exact command a capture runs. Named here so a test can read it."""
    return [
        XVFB,
        "-a",
        "--server-args",
        f"-screen 0 {XVFB_SCREEN}",
        godot,
        "--path",
        str(Path(root) / "client"),
        "--resolution",
        f"{CAPTURE_WINDOW[0]}x{CAPTURE_WINDOW[1]}",
        "-s",
        HARNESS_SCRIPT,
    ]


#: The engine's `class_name` registry, written by `--import`. Ignored by git,
#: so its absence is the normal state of a fresh checkout rather than a fault.
CLASS_CACHE = "client/.godot/global_script_class_cache.cfg"


def preflight(root: Path) -> str:
    """Everything a capture needs, checked before anything is started.

    Returns the client binary. Raises with a reason and a repair when the run
    cannot honestly happen.
    """
    frame = Path(root) / FRAME_FIXTURE
    if not frame.is_file():
        raise CaptureUnavailable(
            f"the recorded frame fixture is missing at {FRAME_FIXTURE}. Record it with: "
            f"tools/run_fixture_land_capture.py --record-frame {FRAME_FIXTURE}"
        )
    godot = os.environ.get(GODOT_VARIABLE, "").strip()
    if not godot:
        raise CaptureUnavailable(
            f"{GODOT_VARIABLE} must name the pinned client binary for a capture to run"
        )
    if shutil.which(godot) is None and not Path(godot).is_file():
        raise CaptureUnavailable(f"{GODOT_VARIABLE} names {godot!r}, which is not an executable")
    if shutil.which(XVFB) is None:
        raise CaptureUnavailable(
            f"{XVFB} is required: a capture needs a real display, and the client's "
            "headless display driver produces no viewport image"
        )
    if not (Path(root) / CLASS_CACHE).is_file():
        # The engine's `class_name` registry is build output: `client/.gitignore`
        # ignores `.godot/`, so a fresh checkout has none and the capture script
        # fails to parse on the first `GridWorldView` it names. Without this
        # check the failure arrives as a parse error inside a launched engine —
        # unmistakably a defect, and not one. Found by the clean-clone proof.
        raise CaptureUnavailable(
            f"the client's class cache is missing at {CLASS_CACHE}; a fresh checkout has "
            "none. Build it with: "
            f"{godot} --headless --path client --import"
        )
    return godot


def request(root: Path, session_directory: Path, capture_id: str | None = None) -> Capture:
    """Run the client harness once and read the three files it wrote."""
    root = Path(root).resolve()
    godot = preflight(root)
    identifier = capture_id or next_capture_id(session_directory)
    output = Path(session_directory) / CAPTURES_DIR / identifier
    output.mkdir(parents=True, exist_ok=True)
    completed = subprocess.run(
        harness_command(root, godot),
        env={
            **os.environ,
            "TME_CAPTURE_FRAME": str(Path(root) / FRAME_FIXTURE),
            "TME_CAPTURE_OUTPUT": str(output),
        },
        capture_output=True,
        text=True,
        check=False,
        timeout=CAPTURE_TIMEOUT_SECONDS,
    )
    if completed.returncode != 0 or SUCCESS_SENTINEL not in completed.stdout:
        detail = completed.stderr.strip()[-800:] or completed.stdout.strip()[-800:]
        raise CaptureUnavailable(
            f"the capture harness failed with status {completed.returncode}: {detail}"
        )
    return load(root, output)
