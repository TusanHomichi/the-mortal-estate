"""Acceptance criteria 8 and 10 — nothing canonical moves, and the comment survives.

Criterion 8 is proven where it actually matters: a full session is taken in
**this** repository, against the real accepted fixture, and `git status` is
compared before and after. A session run in a temporary directory would prove
that temporary directories are safe.

Criterion 10 uses a comment built to break a careless consumer — newlines,
quotes, a trailing space, non-ASCII, and text that looks like structured data.
None of it may be trimmed, normalized, or parsed. It arrives at the agent as
the bytes the owner typed.
"""

from __future__ import annotations

import json
import os
import shutil
import subprocess
import tempfile
import time
import unittest
from pathlib import Path

from workbench_test_support import (
    FIXTURE_ROOT,
    REPO_ROOT,
    StagedTree,
    resolve_json,
)

from workbench import serve
from workbench.packet import build, cells_for_gesture, mask_bytes, now
from workbench.projection import DEFAULT_PROJECTION_PATH
from workbench import session as workbench_session
from workbench.session import (
    COMMENT_RECORD,
    MANIFEST_KIND,
    SELECTION_RECORD,
    SESSION_ROOT,
    attach,
    open_session,
)

#: A comment that punishes any consumer that treats free text as data.
AWKWARD_COMMENT = (
    'move this one cell east — "not" the door.\n'
    "  structure_id=fixture_structure_south x=99 y=99\n"
    "\ttrailing tab and a trailing space \n"
    "…and a non-ASCII ellipsis, plus {\"json\": [1, 2, 3]}"
)


def git(*args: str) -> str:
    return subprocess.run(
        ["git", "-C", str(REPO_ROOT), *args],
        capture_output=True,
        text=True,
        check=True,
    ).stdout


class NothingCanonicalMoves(unittest.TestCase):
    def test_a_full_session_in_this_repository_leaves_the_tree_as_it_was(self) -> None:
        before = git("status", "--porcelain")
        workbench = serve.Workbench(REPO_ROOT, DEFAULT_PROJECTION_PATH, "session-criterion-8")
        directory = REPO_ROOT / workbench.session.relative
        self.addCleanup(shutil.rmtree, directory, ignore_errors=True)

        session = workbench.session
        member = workbench.projection.member("surface")
        for index, (gesture, payload) in enumerate(
            [
                ("click", {"cell": {"x": 8, "y": 6}}),
                ("box", {"rect": {"x": 8, "y": 6, "width": 2, "height": 2}}),
                (
                    "lasso",
                    {
                        "polygon": [
                            {"x": 8.0, "y": 6.0},
                            {"x": 10.0, "y": 6.0},
                            {"x": 10.0, "y": 8.0},
                            {"x": 8.0, "y": 8.0},
                        ]
                    },
                ),
                ("paint", {"cells": [{"x": 8, "y": 8}, {"x": 9, "y": 8}]}),
            ],
            start=1,
        ):
            cells = cells_for_gesture(member, gesture, payload)
            packet = build(
                projection=workbench.projection,
                member=member,
                gesture=gesture,
                cells=cells,
                screen_region={"x": 0, "y": 0, "width": 10, "height": 10},
                comment=AWKWARD_COMMENT,
                selection_id=f"sel-{index:04d}",
                created_at=now(),
                repository_revision=session.manifest["repository_revision"],
                mask_reference=None,
                geometry=payload,
            )
            session.write_selection(
                packet, mask_bytes(member, cells) if gesture in ("lasso", "paint") else None
            )
            session.write_comment(packet["selection_id"], AWKWARD_COMMENT)

        self.assertEqual(len(session.selection_ids()), 4)
        self.assertEqual(git("status", "--porcelain"), before)

    def test_the_session_root_is_ignored_by_git(self) -> None:
        completed = subprocess.run(
            ["git", "-C", str(REPO_ROOT), "check-ignore", "-q", SESSION_ROOT],
            capture_output=True,
        )
        self.assertEqual(completed.returncode, 0, "the session root must be git-ignored")

    def test_the_session_declares_that_it_holds_no_authority(self) -> None:
        manifest = json.loads((FIXTURE_ROOT / "session/manifest.json").read_text())
        self.assertEqual(
            manifest["authority"],
            {
                "tracked_content": False,
                "runtime_input": False,
                "staged_operations": False,
                "apply": False,
            },
        )
        self.assertEqual(manifest["retention"]["policy"], "disposable")
        self.assertIn("Delete any session directory", manifest["retention"]["statement"])


class TheCommentSurvives(StagedTree):
    def setUp(self) -> None:
        super().setUp()
        self.projection = self.staged_projection()
        self.member = self.projection.member("surface")
        self.session = open_session(self.projection, "session-comment")
        geometry = {"cell": {"x": 8, "y": 6}}
        cells = cells_for_gesture(self.member, "click", geometry)
        packet = build(
            projection=self.projection,
            member=self.member,
            gesture="click",
            cells=cells,
            screen_region=None,
            comment=AWKWARD_COMMENT,
            selection_id="sel-0001",
            created_at=now(),
            repository_revision=None,
            mask_reference=None,
            geometry=geometry,
        )
        self.session.write_selection(packet, None)
        self.session.write_comment("sel-0001", AWKWARD_COMMENT)
        self.packet_path = self.staged / self.session.relative / "selections/sel-0001.json"

    def test_the_packet_on_disk_carries_the_comment_byte_for_byte(self) -> None:
        packet = json.loads(self.packet_path.read_text(encoding="utf-8"))
        self.assertEqual(packet["comment"], AWKWARD_COMMENT)
        self.assertEqual(
            packet["comment"].encode("utf-8"), AWKWARD_COMMENT.encode("utf-8")
        )

    def test_the_operation_log_carries_the_comment_byte_for_byte(self) -> None:
        comments = [
            record for record in self.session.operations() if record["kind"] == COMMENT_RECORD
        ]
        self.assertEqual(len(comments), 1)
        self.assertEqual(comments[0]["comment"], AWKWARD_COMMENT)

    def test_the_agent_consumer_hands_the_comment_over_unchanged(self) -> None:
        answer = resolve_json(self.packet_path, self.staged)
        self.assertEqual(answer["comment"], AWKWARD_COMMENT)

    def test_no_fact_in_the_comment_reaches_any_typed_field(self) -> None:
        """The comment names a different structure and impossible coordinates.

        If any of that leaked into the address, a consumer acting on the packet
        would edit the wrong building. Nothing parses the comment, so nothing does.
        """
        packet = json.loads(self.packet_path.read_text(encoding="utf-8"))
        self.assertEqual(packet["cells"], [{"x": 8, "y": 6}])
        self.assertEqual(
            [record["identity"] for record in packet["semantic"] if record["kind"] == "structure"],
            ["structure:surface:fixture_structure_north"],
        )
        typed = dict(packet)
        typed.pop("comment")
        self.assertNotIn("fixture_structure_south", json.dumps(typed))
        addressed = {(cell["x"], cell["y"]) for cell in packet["cells"]}
        for record in packet["semantic"]:
            addressed |= {(cell["x"], cell["y"]) for cell in record["cells"]}
        self.assertEqual(addressed, {(8, 6)})


class TheOperationLogIsANoOpSubstrate(unittest.TestCase):
    """The shape V1 extends, carrying nothing V1 would carry."""

    def setUp(self) -> None:
        self.session = attach(FIXTURE_ROOT, FIXTURE_ROOT / "session")

    def test_the_log_holds_only_selections_and_comments(self) -> None:
        kinds = {record["kind"] for record in self.session.operations()}
        self.assertEqual(kinds, {SELECTION_RECORD, COMMENT_RECORD})

    def test_every_record_carries_a_null_operation_seam(self) -> None:
        for record in self.session.operations():
            with self.subTest(record=record["record_id"]):
                self.assertIn("operation", record)
                self.assertIsNone(record["operation"])

    def test_every_record_is_typed_ordered_and_attributed(self) -> None:
        records = self.session.operations()
        self.assertEqual(
            [record["record_id"] for record in records],
            [f"op-{index:04d}" for index in range(1, len(records) + 1)],
        )
        for record in records:
            self.assertEqual(record["schema_version"], 1)
            self.assertEqual(record["author"], "owner")
            self.assertTrue(record["selection_id"])

    def test_every_selection_record_points_at_a_packet_that_exists(self) -> None:
        for record in self.session.operations():
            if record["kind"] != SELECTION_RECORD:
                continue
            self.assertTrue((FIXTURE_ROOT / "session" / record["packet"]).is_file())

    def test_the_manifest_is_a_session_manifest(self) -> None:
        self.assertEqual(self.session.manifest["kind"], MANIFEST_KIND)
        self.assertEqual(self.session.manifest["view"], "logical")
        self.assertEqual(self.session.manifest["digest_binding"], "fail_closed")
        self.assertEqual(self.session.manifest["revision_binding"], "advisory")


class Retention(unittest.TestCase):
    """The ruling in docs/working-root-policy.md, applied.

    Sessions are disposable and nothing tracked references one, so the only way
    to get this wrong is to delete the session somebody is working in. That is
    the case with its own test.
    """

    def setUp(self) -> None:
        self.root = Path(tempfile.mkdtemp(prefix="tme-retention-")).resolve()
        self.addCleanup(shutil.rmtree, self.root, True)
        self.sessions = self.root / SESSION_ROOT
        self.sessions.mkdir(parents=True)
        self.now = 1_800_000_000.0

    def make(self, name: str, *, days_old: float) -> Path:
        directory = self.sessions / name
        directory.mkdir()
        stamp = self.now - days_old * 86_400
        os.utime(directory, (stamp, stamp))
        return directory

    def test_a_session_older_than_the_window_goes(self) -> None:
        self.make("stale", days_old=20)
        self.make("fresh", days_old=1)
        doomed = [path.name for path in workbench_session.prunable(self.root, at=self.now)]
        self.assertEqual(doomed, ["stale"])

    def test_the_ceiling_removes_the_oldest_beyond_the_limit(self) -> None:
        for index in range(13):
            self.make(f"s{index:02d}", days_old=index * 0.1)
        doomed = {path.name for path in workbench_session.prunable(self.root, at=self.now)}
        self.assertEqual(doomed, {"s10", "s11", "s12"})

    def test_the_session_in_use_is_never_removed(self) -> None:
        self.make("in-use", days_old=400)
        doomed = [
            path.name
            for path in workbench_session.prunable(self.root, keep="in-use", at=self.now)
        ]
        self.assertEqual(doomed, [])

    def test_pruning_actually_removes_the_directories(self) -> None:
        stale = self.make("stale", days_old=30)
        fresh = self.make("fresh", days_old=0)
        removed = workbench_session.prune(self.root, at=self.now)
        self.assertEqual([path.name for path in removed], ["stale"])
        self.assertFalse(stale.exists())
        self.assertTrue(fresh.exists())

    def test_an_absent_session_root_is_not_an_error(self) -> None:
        empty = Path(tempfile.mkdtemp(prefix="tme-retention-empty-"))
        self.addCleanup(shutil.rmtree, empty, True)
        self.assertEqual(workbench_session.prune(empty), [])

    def test_the_manifest_states_the_ruled_numbers(self) -> None:
        self.assertIn(
            str(workbench_session.RETENTION_DAYS), workbench_session.RETENTION_STATEMENT
        )
        self.assertIn(
            str(workbench_session.RETENTION_KEEP), workbench_session.RETENTION_STATEMENT
        )
        self.assertIn("tools/workbench_prune.py", workbench_session.RETENTION_STATEMENT)


if __name__ == "__main__":
    unittest.main()
