"""Shared scaffolding for the Workbench tests.

Two trees are used and the difference matters. The **accepted authoring
fixture** is the real tracked target, and the exact-pointing proofs run against
it because proving a resolver against a toy proves a resolver against a toy.
The **staged tree** is a temporary copy used wherever a proof has to break
something — no mutant ever touches this repository.

The synthetic session fixture under `tests/fixtures/workbench/` is the third
tree: entirely invented, entirely tracked, and the reason a clean clone can
prove the session shape and the agent read path without an ignored root
existing at all (the D6 ruling).

The **capture fixtures** under `tests/fixtures/capture/` are the fourth: two real
captures of one real server frame over the accepted authoring fixture, taken by
the two routes, tracked for the same reason. See their `provenance.md`.
"""

from __future__ import annotations

import json
import shutil
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[1]
TOOLS = REPO_ROOT / "tools"
FIXTURE_ROOT = REPO_ROOT / "tests" / "fixtures" / "workbench"
SYNTHETIC_PROJECTION = "projection/synthetic-logical-projection.json"
RESOLVE = TOOLS / "workbench" / "resolve.py"

if str(TOOLS) not in sys.path:
    sys.path.insert(0, str(TOOLS))

from workbench import capture as capture_reader  # noqa: E402
from workbench.packet import CAPTURE_GEOMETRY_KEYS, build, geometry_of, now  # noqa: E402
from workbench.projection import DEFAULT_PROJECTION_PATH, load  # noqa: E402

#: Every file a selection over the accepted fixture binds itself to. Each one
#: is an independent mutant in the staleness corpus.
BOUND_FILES = (
    "content/authoring-fixture/fixture-surface.tmj",
    "content/authoring-fixture/fixture-interior.tmj",
    "content/authoring-fixture/promotion.json",
    "content/authoring-fixture/generated/world_template.json",
    "content/authoring-fixture/generated/workbench_projection.json",
)

CAPTURE_ROOT = "tests/fixtures/capture"
FIXTURE_ROUTE = f"{CAPTURE_ROOT}/fixture-route"
LIVE_ROUTE = f"{CAPTURE_ROOT}/live-route"
RECORDED_FRAME = f"{CAPTURE_ROOT}/fixture_land_frame.json"

#: The three files a capture selection binds on top of the five above. Each is
#: an independent mutant too: a capture is not one artifact.
CAPTURE_FILES = tuple(
    f"{FIXTURE_ROUTE}/{name}"
    for name in ("capture.png", "capture.identity.pgm", "capture.sidecar.json")
)


def fixture_route_capture():
    """The tracked capture taken by the ordinary route."""
    return capture_reader.load(REPO_ROOT, REPO_ROOT / FIXTURE_ROUTE)


def live_route_capture():
    """The tracked capture taken by the accuracy reference."""
    return capture_reader.load(REPO_ROOT, REPO_ROOT / LIVE_ROUTE)


def recorded_frame() -> dict:
    return json.loads((REPO_ROOT / RECORDED_FRAME).read_text(encoding="utf-8"))


def square_rect(taken, x: int, y: int) -> dict:
    """The rectangle one world square occupies in a capture, from its sidecar.

    Read off the target list rather than recomputed from the camera, so a test
    that compares two framings compares what each presenter actually drew.
    """
    for record in taken.targets:
        if record["identity"] == f"tile:{x}:{y}":
            return dict(record["hit_shape"])
    raise AssertionError(f"the capture does not show square {x},{y}")


def region_rect(taken, x0: int, y0: int, x1: int, y1: int) -> dict:
    """The pixel rectangle covering an inclusive block of world squares."""
    first = square_rect(taken, x0, y0)
    last = square_rect(taken, x1, y1)
    return {
        "x": first["x"],
        "y": first["y"],
        "width": last["x"] + last["width"] - first["x"],
        "height": last["y"] + last["height"] - first["y"],
    }


def capture_packet(
    projection,
    taken,
    gesture: str,
    body: dict,
    *,
    root=None,
    selection_id: str = "sel-0001",
    comment: str = "",
) -> dict:
    """Build one capture packet through the same path the server uses."""
    geometry = geometry_of(gesture, body, CAPTURE_GEOMETRY_KEYS)
    selection = capture_reader.select(projection, taken, gesture, geometry)
    return build(
        projection=projection,
        member=selection["member"],
        gesture=gesture,
        cells=selection["cells"],
        screen_region=capture_reader.canvas_rect(gesture, geometry),
        comment=comment,
        selection_id=selection_id,
        created_at=now(),
        repository_revision=None,
        mask_reference=None,
        geometry=geometry,
        capture=taken.binding(root or REPO_ROOT, selection["observed"]),
    )


def accepted_projection():
    """The tracked logical projection of the accepted authoring fixture."""
    return load(REPO_ROOT, DEFAULT_PROJECTION_PATH)


def synthetic_projection():
    """The tracked synthetic projection the session fixture was taken against."""
    return load(FIXTURE_ROOT, SYNTHETIC_PROJECTION)


def surface():
    return accepted_projection().member("surface")


def run_resolve(packet: Path, root: Path, extra: list[str] | None = None):
    """Run the agent-facing consumer as an agent would: as a program."""
    return subprocess.run(
        [sys.executable, str(RESOLVE), str(packet), "--root", str(root), *(extra or [])],
        capture_output=True,
        text=True,
        cwd=REPO_ROOT,
    )


def resolve_json(packet: Path, root: Path) -> dict:
    completed = run_resolve(packet, root, ["--json"])
    if completed.returncode != 0:
        raise AssertionError(f"resolve refused: {completed.stderr}")
    return json.loads(completed.stdout)


class StagedTree(unittest.TestCase):
    """A test case with a throwaway copy of the accepted fixture to break."""

    #: Subclasses that need a capture to break as well extend this.
    staged_files = BOUND_FILES

    def setUp(self) -> None:
        super().setUp()
        self.staged = Path(tempfile.mkdtemp(prefix="tme-workbench-")).resolve()
        self.addCleanup(shutil.rmtree, self.staged, ignore_errors=True)
        for relative in self.staged_files:
            target = self.staged / relative
            target.parent.mkdir(parents=True, exist_ok=True)
            shutil.copy(REPO_ROOT / relative, target)

    def staged_projection(self):
        return load(self.staged, DEFAULT_PROJECTION_PATH)

    def corrupt(self, relative: str) -> None:
        """Change one bound file's bytes without changing what it is.

        The mutation is a whitespace-only reformat: the document still parses,
        still means the same thing, and only its digest moves. That is the
        mutant worth having — a consumer that only notices catastrophic damage
        is not a consumer that notices drift.
        """
        path = self.staged / relative
        if path.suffix in (".png", ".pgm"):
            # A picture and a raster are binary; the equivalent minimal mutation
            # is one byte, which is the drift a digest exists to catch.
            payload = bytearray(path.read_bytes())
            payload[-1] ^= 0x01
            path.write_bytes(bytes(payload))
            return
        document = json.loads(path.read_text(encoding="utf-8"))
        path.write_text(json.dumps(document, indent=4) + "\n", encoding="utf-8")
