#!/usr/bin/env python3
"""Drive the Workbench end to end, the way an owner and an agent actually would.

Starts the local server on a free loopback port, takes one click selection and
one lasso selection over the logical view, takes the same selection again over a
real gameplay capture, shows the packets that were written, resolves them through
the agent-facing consumer with nothing but file reads — and then, in V1's half,
stages a typed edit to the map and a typed edit to a picture, previews the
candidate they produce, applies both atomically, and records the owner's
acceptance as an intent that grants nothing.

The capture half is the point of the second act: the same square, pointed at in a
photograph of the running game instead of in the compiler's projection, comes back
with the same address. That is acceptance criterion 2, shown rather than asserted.

The V1 half is the point of the third: one map edit and one visual edit, staged
from a selection, judged by the compiler's own semantics, applied together or not
at all, and — deliberately shown — one Apply that is REFUSED, leaving the tree
exactly as it was.

The session is seeded with the tracked capture fixture so this runs anywhere. When
the pinned client and a virtual display are available it **also** takes a fresh
capture through the real request route, and says which one it is pointing at.

This is a demonstration, not a test — the tests live in `tests/`. It exists so the
whole loop can be seen working in one output, which is a different kind of
evidence from an assertion passing.

    python3 tools/workbench_demo.py

Exit codes: 0 when the whole loop ran, 1 when any step failed.
"""

from __future__ import annotations

import hashlib
import json
import shutil
import subprocess
import sys
import time
import urllib.error
import urllib.request
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))

from workbench import capture as capture_reader  # noqa: E402
from workbench import capture_harness  # noqa: E402
from workbench import imageops  # noqa: E402
from workbench_integrity import carried_tree  # noqa: E402

ROOT = Path(__file__).resolve().parents[1]
STARTUP_TIMEOUT_SECONDS = 15
SESSION_ID = "session-demo"
TRACKED_CAPTURE = "tests/fixtures/capture/fixture-route"

#: The tracked synthetic editable master the visual edit works on, and its
#: digest as the provenance record carries it. Both are read from that record
#: rather than restated, so an edited master fails closed here too.
ASSET_PROVENANCE = "content/authoring-fixture/asset-provenance.json"

#: The fixture's arrival square: a landmark standing on a route, so both views
#: have something genuinely ambiguous to agree about.
ARRIVAL = {"x": 12, "y": 14}


def rule(title: str) -> None:
    print(f"\n=== {title} " + "=" * max(0, 66 - len(title)))


def post(base: str, path: str, body: dict) -> dict:
    request = urllib.request.Request(
        base + path,
        data=json.dumps(body).encode("utf-8"),
        headers={"Content-Type": "application/json"},
        method="POST",
    )
    with urllib.request.urlopen(request, timeout=10) as response:
        return json.loads(response.read())


def get(base: str, path: str) -> dict:
    with urllib.request.urlopen(base + path, timeout=10) as response:
        return json.loads(response.read())


def seed_session() -> Path:
    """Hand the session the tracked capture, so this runs on any machine."""
    session = ROOT / ".workbench/sessions" / SESSION_ID
    shutil.rmtree(session, ignore_errors=True)
    destination = session / capture_reader.CAPTURES_DIR / "cap-0001"
    destination.parent.mkdir(parents=True, exist_ok=True)
    shutil.copytree(ROOT / TRACKED_CAPTURE, destination)
    return session


def start_server() -> tuple[subprocess.Popen, str]:
    process = subprocess.Popen(
        [sys.executable, "tools/workbench/serve.py", "--port", "0", "--session", SESSION_ID],
        cwd=ROOT,
        stdout=subprocess.PIPE,
        stderr=subprocess.DEVNULL,
        text=True,
    )
    deadline = time.monotonic() + STARTUP_TIMEOUT_SECONDS
    base = None
    while time.monotonic() < deadline:
        line = process.stdout.readline()
        if not line:
            break
        print("  " + line.rstrip())
        if "serving" in line:
            base = line.split()[-1].rstrip("/")
        if "bound to" in line and base:
            return process, base
    process.kill()
    raise SystemExit("the workbench did not start")


def anchor_from(base: str, capture_id: str) -> dict:
    """The anchor pixel of the arrival square, read off the served capture."""
    state = get(base, "/api/state")
    directory = next(
        row["directory"] for row in state["captures"] if row["capture_id"] == capture_id
    )
    sidecar = json.loads((ROOT / directory / "capture.sidecar.json").read_text())
    return next(
        record["anchor"]
        for record in sidecar["targets"]
        if record["identity"] == f"tile:{ARRIVAL['x']}:{ARRIVAL['y']}"
    )


def summarize(packet: dict) -> None:
    print(f"  selection_id  {packet['selection_id']}")
    print(f"  view          {packet['view']}  member {packet['scene']['member']}"
          + (f"  frame generation {packet['scene']['frame_generation']}"
             if packet["scene"]["frame_generation"] is not None else ""))
    print(f"  gesture       {packet['screen_region']['gesture']}")
    print(f"  cells         {len(packet['cells'])}: " +
          " ".join(f"{cell['x']},{cell['y']}" for cell in packet["cells"][:12]))
    print(f"  ambiguous     {packet['ambiguous']}")
    for record in packet["semantic"]:
        coverage = record["coverage"]
        print(
            f"    {record['rank']:>2}. {record['kind']:<12} {record['identity']}"
            f"  selection={coverage['selection_coverage']:.2f}"
            f" identity={coverage['identity_coverage']:.2f}"
        )
    mask = packet["screen_region"].get("mask")
    if mask:
        print(f"  mask          {mask['path']}  {mask['sha256'][:16]}")
    if packet["observed"]:
        print("  observed      " + ", ".join(
            f"{row['kind']} {row['identity']}" for row in packet["observed"]
        ))
    print(f"  bound         {len(packet['source']['digests'])} digests, fail-closed")
    print(f"  comment       {packet['comment']!r}")


def capture_section(base: str) -> str:
    """Point at a photograph of the running game, and say which one."""
    rule("the capture the owner is pointing at")
    try:
        capture_harness.preflight(ROOT)
    except capture_reader.CaptureUnavailable as unavailable:
        print(f"  a fresh capture is unavailable: {unavailable}")
        print("  pointing at the tracked capture fixture instead (cap-0001)")
        return "cap-0001"
    started = time.monotonic()
    answer = post(base, "/api/capture", {})["capture"]
    print(f"  took a fresh capture in {time.monotonic() - started:.1f}s")
    for field in ("capture_id", "directory", "member", "frame_generation", "viewport", "targets"):
        print(f"  {field:<17} {answer[field]}")
    print(f"  square pitch      {answer['camera']['square_pitch_px']}px")
    return answer["capture_id"]


def digests_of(root: Path) -> dict:
    """The tracked editable master, named by the record that vouches for it."""
    document = json.loads((root / ASSET_PROVENANCE).read_text())
    return document["assets"][0]


def stage(base: str, body: dict) -> dict:
    return post(base, "/api/stage", body)["record"]


def truth_act(base: str, selection_id: str) -> dict:
    rule("the owner stages ONE map edit, from the selection they just made")
    record = stage(
        base,
        {
            "selection_id": selection_id,
            "verb": "move_landmark",
            "parameters": {"landmark_id": "fixture_ruin_marker", "to": {"x": 6, "y": 11}},
            "comment": "the ruin marker reads one cell too far west",
        },
    )
    operation = record["operation"]
    print(f"  {record['record_id']}  {operation['class']:<6} {operation['verb']}"
          f"  from {record['selection_id']}  by {record['author']}")
    print(f"  parameters    {json.dumps(operation['parameters'])}")
    print("  staged        nothing tracked has changed; a session is a log")
    return record


def asset_act(base: str, session: Path, selection_id: str) -> dict:
    rule("the owner stages ONE visual edit, on the tracked editable master")
    asset = digests_of(ROOT)
    mask_pixels = [(10 + dx, 10 + dy) for dy in range(4) for dx in range(6)]
    mask_path = session / "commit" / "commit-demo.pbm"
    mask_path.parent.mkdir(parents=True, exist_ok=True)
    payload = imageops.write_mask(mask_pixels)
    mask_path.write_bytes(payload)
    record = stage(
        base,
        {
            "selection_id": selection_id,
            "class": "asset",
            "member": "asset",
            "verb": "edit_region",
            "parameters": {
                "source": {"path": asset["path"], "sha256": asset["sha256"]},
                "commit_mask": {
                    "path": str(mask_path.relative_to(ROOT)),
                    "sha256": hashlib.sha256(payload).hexdigest(),
                },
                "context": {"margin": 3},
            },
            "adapter": {"adapter": "palette_fill", "parameters": {"colour": [26, 26, 26, 255]}},
            "comment": "darken this patch",
        },
    )
    print(f"  {record['record_id']}  asset   edit_region  on {asset['path']}")
    print(f"  commit mask   {len(mask_pixels)} pixels — the ONLY pixels that may change")
    print(f"  context       a 3px margin around them, which the adapter may SEE and not keep")
    print(f"  adapter       palette_fill, local and deterministic; it knows nothing about the mask")
    return record


def preview_act(base: str) -> dict:
    rule("preview: the candidate the whole staged log produces")
    answer = post(base, "/api/candidate", {})
    outcome = answer["outcome"]
    print(f"  stage         {outcome['stage']}")
    print(f"  base          {outcome['base']['sha256'][:16]}  {outcome['base']['path']}")
    if outcome["candidate"]:
        print(f"  candidate     {outcome['candidate']['sha256'][:16]}  {outcome['candidate']['path']}")
    if outcome["accepted"]:
        statistics = outcome["detail"]["statistics"]
        print(f"  ACCEPTED      {statistics['passable_cells']} walkable cells, "
              f"{statistics['route_cells']} route cells")
        accepted = get(base, "/api/projection")
        changed = differing_cells(accepted, answer["projection"])
        print(f"  differs in    {len(changed)} cell(s): "
              + " ".join(f"{x},{y}" for x, y in changed))
    return answer


def differing_cells(accepted: dict, candidate: dict) -> list:
    """Which cells the candidate changed. A comparison of two documents the
    compiler wrote, not a second opinion about either."""
    if candidate is None:
        return []
    changed = []
    for member in candidate["members"]:
        was = next(row for row in accepted["members"] if row["member"] == member["member"])
        before = {(cell["x"], cell["y"]): cell for cell in was["cells"]}
        for cell in member["cells"]:
            key = (cell["x"], cell["y"])
            if before[key] != cell:
                changed.append(key)
    return sorted(changed, key=lambda cell: (cell[1], cell[0]))


def apply_act(base: str) -> dict:
    rule("Apply — atomic, and not promotion")
    answer = post(base, "/api/apply", {})
    record = answer["record"]
    print(f"  {'APPLIED' if answer['accepted'] else 'REJECTED'}       {answer['apply_id']}"
          f"  {answer['path']}")
    for output in record["outputs"]:
        print(f"  {output['role']:<22} {output['sha256'][:16]}  {output['path']}")
    for edit in record["asset_edits"]:
        touched = edit["adapter_wrote_outside_the_mask"]
        print(f"  visual edit   {edit['changed_pixels']} pixels changed, inside the commit mask")
        print(f"  preserved     the adapter also painted {touched['pixels']} pixels outside it; "
              "every one of them was discarded")
    print(f"  grants        {json.dumps(record['grants'])}")
    return answer


def rejection_act(base: str, selection_id: str) -> None:
    rule("and one Apply that is REFUSED — which is the more important half")
    tree = carried_tree(ROOT)
    record = stage(
        base,
        {
            "selection_id": selection_id,
            "verb": "move_landmark",
            "parameters": {"landmark_id": "fixture_ruin_marker", "to": {"x": 0, "y": 0}},
            "comment": "put it in the sea",
        },
    )
    print(f"  staged        {record['record_id']}  move_landmark to 0,0 — open water")
    answer = post(base, "/api/apply", {})
    rejection = answer["record"]
    print(f"  REJECTED      {answer['apply_id']}  at stage {rejection['stage']}")
    print(f"  operation     {rejection['operation']['record_id']}  {rejection['operation']['verb']}")
    print(f"  assertion     {rejection['assertion']}")
    print(f"  record        {answer['path']}")
    after = carried_tree(ROOT)
    changed = sorted(name for name in tree.keys() | after.keys() if tree.get(name) != after.get(name))
    if changed:
        raise RuntimeError(f"rejected Apply changed carried files: {', '.join(changed)}")
    print("  carried tree  UNCHANGED")
    post(base, "/api/retract", {"record_id": record["record_id"], "reason": "wrong cell"})
    print(f"  retracted     {record['record_id']} — the log keeps what was tried")


def accept_act(base: str, applied: dict) -> None:
    rule("the owner accepts the candidate — as an intent, and nothing more")
    receipt = applied["record"]
    candidate = next(
        output for output in receipt["outputs"] if output["role"] == "candidate_master"
    )
    answer = post(
        base,
        "/api/accept",
        {
            "candidate_sha256": candidate["sha256"],
            "apply_id": applied["apply_id"],
            "note": "the marker sits right now",
        },
    )
    record = answer["record"]
    print(f"  {record['record_id']}  candidate {record['candidate_sha256'][:16]}")
    print(f"  grants        {json.dumps(record['grants'])}")
    print(f"  ceremony      {record['ceremony']}")


def main() -> int:
    rule("start the local workbench on a free loopback port")
    seed_session()
    process, base = start_server()
    try:
        rule("the owner clicks one cell")
        click = post(
            base,
            "/api/selection",
            {
                "member": "surface",
                "gesture": "click",
                "cell": {"x": 8, "y": 6},
                "comment": "this building sits one cell too far north",
            },
        )["packet"]
        summarize(click)

        rule("the owner lassos the crossing")
        lasso = post(
            base,
            "/api/selection",
            {
                "member": "surface",
                "gesture": "lasso",
                "polygon": [
                    {"x": 11.0, "y": 8.0},
                    {"x": 15.0, "y": 8.0},
                    {"x": 15.0, "y": 11.0},
                    {"x": 11.0, "y": 11.0},
                ],
                "comment": "the junction reads too wide here",
            },
        )["packet"]
        summarize(lasso)

        capture_id = capture_section(base)

        rule("the owner points at the SAME square, over the capture")
        logical_arrival = post(
            base,
            "/api/preview",
            {"member": "surface", "gesture": "click", "cell": ARRIVAL},
        )
        anchor = anchor_from(base, capture_id)
        capture_click = post(
            base,
            "/api/capture/selection",
            {
                "capture_id": capture_id,
                "gesture": "click",
                "point": anchor,
                "comment": "the same square, pointed at in a photograph",
            },
        )["packet"]
        summarize(capture_click)
        print(f"  pixel pointed at   {anchor['x']},{anchor['y']}")
        print(
            "  SAME ADDRESS       "
            + str(capture_click["semantic"] == logical_arrival["semantic"]
                  and capture_click["cells"] == logical_arrival["cells"])
        )

        rule("what the session directory holds")
        state = get(base, "/api/state")
        directory = ROOT / state["session_directory"]
        for path in sorted(directory.rglob("*")):
            if path.is_file():
                print(f"  {path.relative_to(ROOT)}  ({path.stat().st_size} bytes)")

        rule("the packet on disk, as an agent reads it")
        packet_path = directory / "selections" / f"{lasso['selection_id']}.json"
        print(json.dumps(json.loads(packet_path.read_text())["screen_region"], indent=2))

        rule("the agent resolves it with file reads only — no browser, no server")
        resolved = subprocess.run(
            [sys.executable, "tools/workbench/resolve.py", str(packet_path)],
            cwd=ROOT,
            capture_output=True,
            text=True,
        )
        print(resolved.stdout.rstrip())
        if resolved.returncode != 0:
            print(resolved.stderr.rstrip(), file=sys.stderr)
            return 1

        truth_act(base, click["selection_id"])
        asset_act(base, directory, lasso["selection_id"])
        preview_act(base)
        applied = apply_act(base)
        accept_act(base, applied)
        rejection_act(base, click["selection_id"])

        rule("the operation log — one file, one order, every kind of record")
        for line in (directory / "operations.jsonl").read_text().splitlines():
            record = json.loads(line)
            operation = record.get("operation")
            detail = f"{operation['class']}:{operation['verb']}" if operation else "-"
            print(
                f"  {record['record_id']}  {record['kind']:<20} "
                f"selection={str(record['selection_id']):<9} operation={detail}"
            )
        return 0
    finally:
        rule("stop the workbench")
        process.terminate()
        try:
            process.wait(timeout=5)
        except subprocess.TimeoutExpired:
            process.kill()
        print("  stopped")


if __name__ == "__main__":
    sys.exit(main())
