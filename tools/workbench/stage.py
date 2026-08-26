#!/usr/bin/env python3
"""Stage operations from a terminal: the whole V1 loop, no browser.

Agent parity is a law: nothing the Workbench can do is unavailable to an agent
working on the same session files. That law is kept two ways, and both are real.

**With file tools alone.** A session is plain files. Staging an operation is
appending one JSON line to `operations.jsonl`; retracting one is appending
another. An agent that would rather write the record itself needs nothing from
this file — read `docs/workbench-v1.md` for the record shape and write it.

**With one command.** This is that command, for when writing JSON by hand is
just friction:

    python3 tools/workbench/stage.py open
    python3 tools/workbench/stage.py verbs --session <session>
    python3 tools/workbench/stage.py point --session <session> --click 6,11
    python3 tools/workbench/stage.py add --session <session> --selection sel-0001 \\
        --verb move_landmark --parameters '{"landmark_id":"fixture_ruin_marker","to":{"x":6,"y":11}}'
    python3 tools/workbench/stage.py list --session <session>
    python3 tools/workbench/stage.py status --session <session>
    python3 tools/workbench/stage.py preview --session <session>
    python3 tools/workbench/apply.py <session>

Every staged operation names the selection packet it derives from, and `point`
is how an agent gets one. That requirement is not ceremony: the packet is what
binds the edit to a set of digests, and an operation with no packet is an agent
asserting coordinates it typed rather than an act upon an address.

Exit codes: 0 done, 2 refused, 3 the session or the tree could not be read.
"""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path

if __package__ in (None, ""):  # invoked as a script, the ordinary agent path
    sys.path.insert(0, str(Path(__file__).resolve().parents[1]))

from workbench import bridge  # noqa: E402
from workbench import imageops  # noqa: E402
from workbench import operations as operation_log  # noqa: E402
from workbench import replay as replay_module  # noqa: E402
from workbench.packet import now  # noqa: E402
from workbench.projection import DEFAULT_PROJECTION_PATH, WorkbenchError, load  # noqa: E402
from workbench.resolve import find_root  # noqa: E402
from workbench.session import COMMIT_MASKS_DIR, SESSION_ROOT, attach, open_session  # noqa: E402

EXIT_OK = 0
EXIT_REFUSED = 2
EXIT_UNREADABLE = 3

DEFAULT_AUTHOR = "agent"


def _session(root: Path, name: str):
    directory = Path(name)
    if not directory.is_absolute() and not directory.exists():
        directory = root / SESSION_ROOT / name
    return attach(root, directory)


def _cell(text: str) -> dict:
    x, _, y = text.partition(",")
    return {"x": int(x), "y": int(y)}


def command_open(root: Path, arguments) -> int:
    projection = load(root, arguments.projection)
    session = open_session(projection, arguments.name)
    print(session.relative)
    return EXIT_OK


def command_verbs(root: Path, arguments) -> int:
    """The whole vocabulary, from the two places that own its halves.

    Truth verbs come from the authoring compiler, which declares them, parses
    them, and judges their result. Asset verbs come from the project's image
    operations. Dressing has none, and says why. Nothing here restates either
    table — a third copy would be a third thing to keep true.
    """
    projection = load(root, arguments.projection)
    truth = bridge.describe_operations(root, projection.land_id)
    document = {
        operation_log.CLASS_TRUTH: truth,
        operation_log.CLASS_DRESSING: {"verbs": [], "ruling": operation_log.DRESSING_RULING},
        operation_log.CLASS_ASSET: {
            "verbs": [
                {
                    "verb": contract.name,
                    "summary": contract.summary,
                    "required": list(contract.required),
                    "optional": list(contract.optional),
                    "adapters": list(contract.adapter_kinds),
                }
                for contract in imageops.CONTRACTS.values()
            ],
            "adapters": sorted(imageops.REGISTRY),
        },
    }
    if arguments.json:
        print(json.dumps(document, indent=2))
        return EXIT_OK
    for spec in truth["verbs"]:
        print(f"truth     {spec['verb']}")
        print(f"            {spec['summary']}")
        for parameter in spec["parameters"]:
            print(f"            {parameter['name']}: {parameter['shape']} — {parameter['summary']}")
        print(f"            rejects: {spec['rejects']}")
    print(f"dressing  (none) — {operation_log.DRESSING_RULING}")
    for contract in imageops.CONTRACTS.values():
        served = "adapter registered" if contract.name == "edit_region" else "no adapter registered"
        print(f"asset     {contract.name}  ({served})")
        print(f"            {contract.summary}")
    return EXIT_OK


def command_point(root: Path, arguments) -> int:
    """Write one selection packet, the way the browser writes one."""
    session = _session(root, arguments.session)
    projection = load(root, arguments.projection)
    body: dict = {
        "member": arguments.member or session.candidate_member,
        "comment": arguments.comment,
        "author": arguments.author,
    }
    if arguments.click:
        body |= {"gesture": "click", "cell": _cell(arguments.click)}
    elif arguments.box:
        x, y, width, height = (int(part) for part in arguments.box.split(","))
        body |= {
            "gesture": "box",
            "rect": {"x": x, "y": y, "width": width, "height": height},
        }
    elif arguments.cells:
        body |= {
            "gesture": "paint",
            "cells": [_cell(part) for part in arguments.cells.split()],
        }
    else:
        raise WorkbenchError("point needs one of --click, --box, or --cells")
    packet = session.record_logical_selection(projection, body)
    print(packet["selection_id"])
    return EXIT_OK


def command_mask(root: Path, arguments) -> int:
    """Write one commit mask into the session, in the asset's pixel space.

    A commit mask is the exact set of pixels an asset edit may replace, and
    naming it is an owner act — widening it later is a separate operation with
    its own record. It lives in the session beside the packets, is bound by
    digest like every other input, and is never tracked.

    **The honest limit of this slice:** the mask is given as a rectangle here,
    not drawn over a picture. V1 ships the asset operation, the adapter contract,
    and the preservation rule; an asset VIEW to point at pixels in is the slice
    that comes after, and until it exists a commit mask is stated rather than
    gestured at.
    """
    session = _session(root, arguments.session)
    x, y, width, height = (int(part) for part in arguments.rect.split(","))
    if width <= 0 or height <= 0:
        raise WorkbenchError("a commit mask covers at least one pixel")
    payload = imageops.write_mask(
        [(x + dx, y + dy) for dy in range(height) for dx in range(width)]
    )
    record = session.write_artifact(f"{COMMIT_MASKS_DIR}/{arguments.name}.pbm", payload)
    print(json.dumps(record, indent=2))
    return EXIT_OK


def command_add(root: Path, arguments) -> int:
    session = _session(root, arguments.session)
    session.read_packet(arguments.selection)  # refuses now rather than at Apply
    parameters = json.loads(arguments.parameters)
    adapter = json.loads(arguments.adapter) if arguments.adapter else None
    record = operation_log.build(
        record_id=session.next_record_id(),
        recorded_at=now(),
        author=arguments.author,
        selection_id=arguments.selection,
        operation_class=arguments.operation_class,
        member=arguments.member or session.candidate_member,
        editable_member=session.candidate_member,
        verb=arguments.verb,
        parameters=parameters,
        adapter=adapter,
        comment=arguments.comment,
    )
    session.stage_operation(record)
    print(record["record_id"])
    return EXIT_OK


def command_retract(root: Path, arguments) -> int:
    session = _session(root, arguments.session)
    record = session.retract_operation(arguments.record, arguments.reason, arguments.author)
    print(record["record_id"])
    return EXIT_OK


def command_list(root: Path, arguments) -> int:
    session = _session(root, arguments.session)
    staged = session.staged()
    if arguments.json:
        print(json.dumps(operation_log.summary(staged), indent=2))
        return EXIT_OK
    print(f"session    {session.relative}")
    print(f"staged     {len(staged)} operation(s), in log order")
    for record in staged:
        operation = record["operation"]
        print(
            f"  {record['record_id']}  {operation['class']:<6} {operation['verb']:<24}"
            f"  from {record['selection_id']}  by {record['author']}"
        )
        print(f"      {json.dumps(operation['parameters'])}")
    return EXIT_OK


def command_status(root: Path, arguments) -> int:
    """The whole session at a glance, for an agent handed one it did not open.

    What it is bound to, what is staged, and what every Apply said — with the
    receipt or the rejection named by path so the next question is a file read.
    Everything here is already plain JSON on disk; this is the one command that
    saves an agent three of them.
    """
    session = _session(root, arguments.session)
    log = session.operations()
    applies = [row for row in log if row.get("kind") == operation_log.APPLY_RECORDED]
    accepted = [row for row in log if row.get("kind") == operation_log.CANDIDATE_ACCEPTED]
    document = {
        "session": session.relative,
        "opened_at": session.manifest.get("opened_at"),
        "authority": session.manifest.get("authority"),
        "base_digests": session.manifest.get("base_digests", []),
        "selections": session.selection_ids(),
        "staged": operation_log.summary(session.staged()),
        "applies": [
            {
                "apply_id": row["apply_id"],
                "outcome": row["outcome"],
                "record": row["record"],
                "recorded_at": row["recorded_at"],
            }
            for row in applies
        ],
        "candidate_acceptances": [
            {
                "record_id": row["record_id"],
                "candidate_sha256": row["candidate_sha256"],
                "apply_id": row["apply_id"],
                "grants": row["grants"],
            }
            for row in accepted
        ],
    }
    if arguments.json:
        print(json.dumps(document, indent=2))
        return EXIT_OK
    print(f"session     {document['session']}   opened {document['opened_at']}")
    print(f"authority   {json.dumps(document['authority'])}")
    print(f"bound       {len(document['base_digests'])} digests, fail-closed")
    for record in document["base_digests"]:
        print(f"              {record['role']:<20} {record['sha256'][:12]}  {record['path']}")
    print(f"selections  {', '.join(document['selections']) or '(none)'}")
    print(f"staged      {len(document['staged'])} operation(s), in log order")
    for record in document["staged"]:
        print(f"  {record['record_id']}  {record['class']:<6} {record['verb']:<24}"
              f"  from {record['selection_id']}  by {record['author']}")
    print(f"applies     {len(document['applies'])}")
    for record in document["applies"]:
        print(f"  {record['apply_id']}  {record['outcome']:<9} {record['record']}")
    for record in document["candidate_acceptances"]:
        print(f"  accepted  {record['candidate_sha256'][:12]}  from {record['apply_id']}"
              f"  grants {json.dumps(record['grants'])}")
    return EXIT_OK


def command_preview(root: Path, arguments) -> int:
    """Replay the staged set and say what the compiler makes of it.

    The same replay Apply runs, into the session's own preview directory,
    writing no receipt. A preview that used a different path would be a preview
    of something other than what Apply would do.
    """
    session = _session(root, arguments.session)
    outcome = replay_module.preview(session)
    if arguments.json:
        print(json.dumps(outcome.as_record(), indent=2))
        return EXIT_OK if outcome.accepted else EXIT_REFUSED
    print(f"stage      {outcome.stage}")
    if outcome.candidate:
        print(f"candidate  {outcome.candidate['sha256'][:12]}  {outcome.candidate['path']}")
    if outcome.projection:
        print(f"projection {outcome.projection['sha256'][:12]}  {outcome.projection['path']}")
    if outcome.accepted:
        statistics = outcome.detail.get("statistics") or {}
        print(
            f"ACCEPTED   {statistics.get('member')} · "
            f"{statistics.get('passable_cells')} walkable cells, "
            f"{statistics.get('route_cells')} route cells, "
            f"{statistics.get('structure_footprint_cells')} footprint cells"
        )
        return EXIT_OK
    print("REJECTED")
    for line in json.dumps(outcome.detail, indent=2).splitlines():
        print(f"  {line}")
    return EXIT_REFUSED


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--root", default=None, help="repository root (default: inferred)")
    parser.add_argument("--projection", default=DEFAULT_PROJECTION_PATH)
    commands = parser.add_subparsers(dest="command", required=True)

    opened = commands.add_parser("open", help="create a session")
    opened.add_argument("--name", default=None)
    opened.set_defaults(handler=command_open)

    verbs = commands.add_parser("verbs", help="the whole operation vocabulary")
    verbs.add_argument("--json", action="store_true")
    verbs.set_defaults(handler=command_verbs)

    point = commands.add_parser("point", help="write one selection packet")
    point.add_argument("--session", required=True)
    point.add_argument(
        "--member",
        default=None,
        help="default: the member the session's land accepts truth operations against",
    )
    point.add_argument("--click", default=None, metavar="X,Y")
    point.add_argument("--box", default=None, metavar="X,Y,W,H")
    point.add_argument("--cells", default=None, metavar='"X,Y X,Y"')
    point.add_argument("--comment", default="")
    point.add_argument("--author", default=DEFAULT_AUTHOR)
    point.set_defaults(handler=command_point)

    mask = commands.add_parser("mask", help="write one commit mask for an asset edit")
    mask.add_argument("--session", required=True)
    mask.add_argument("--rect", required=True, metavar="X,Y,W,H", help="in source image pixels")
    mask.add_argument("--name", default="commit-0001")
    mask.set_defaults(handler=command_mask)

    add = commands.add_parser("add", help="stage one operation")
    add.add_argument("--session", required=True)
    add.add_argument("--selection", required=True)
    add.add_argument("--verb", required=True)
    add.add_argument("--parameters", required=True, help="a JSON object")
    add.add_argument("--adapter", default=None, help="a JSON object, for asset operations")
    add.add_argument("--class", dest="operation_class", default=operation_log.CLASS_TRUTH)
    add.add_argument(
        "--member",
        default=None,
        help="default: the member the session's land accepts truth operations against",
    )
    add.add_argument("--comment", default="")
    add.add_argument("--author", default=DEFAULT_AUTHOR)
    add.set_defaults(handler=command_add)

    retract = commands.add_parser("retract", help="retract one staged operation")
    retract.add_argument("--session", required=True)
    retract.add_argument("--record", required=True)
    retract.add_argument("--reason", default="")
    retract.add_argument("--author", default=DEFAULT_AUTHOR)
    retract.set_defaults(handler=command_retract)

    listing = commands.add_parser("list", help="the effective staged set")
    listing.add_argument("--session", required=True)
    listing.add_argument("--json", action="store_true")
    listing.set_defaults(handler=command_list)

    status = commands.add_parser("status", help="the whole session at a glance")
    status.add_argument("--session", required=True)
    status.add_argument("--json", action="store_true")
    status.set_defaults(handler=command_status)

    preview = commands.add_parser("preview", help="replay the staged set and judge it")
    preview.add_argument("--session", required=True)
    preview.add_argument("--json", action="store_true")
    preview.set_defaults(handler=command_preview)
    return parser


def main(argv: list[str] | None = None) -> int:
    arguments = build_parser().parse_args(argv)
    try:
        root = (
            Path(arguments.root).resolve()
            if arguments.root
            else find_root(Path(__file__).resolve().parent)
        )
        return arguments.handler(root, arguments)
    except WorkbenchError as error:
        print(f"REFUSED: {error}", file=sys.stderr)
        return EXIT_REFUSED
    except (OSError, json.JSONDecodeError, ValueError) as error:
        print(f"UNREADABLE: {error}", file=sys.stderr)
        return EXIT_UNREADABLE


if __name__ == "__main__":
    sys.exit(main())
