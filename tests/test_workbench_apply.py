"""Apply: deterministic, atomic, and not promotion.

Three claims are worth more than everything else in this file, and each is
proven by a mutant rather than by reading the code:

1. **Deterministic.** The same log against the same base produces byte-identical
   output — and an EMPTY log reproduces the accepted master byte for byte, which
   is a stronger statement than it looks: it means the round trip through the
   compiler's serializer is the identity on the tracked bytes, so a candidate
   differs from the master exactly where the operations say and nowhere else.
2. **Atomic.** A rejected Apply leaves the tracked tree byte-identical and writes
   exactly one new file — the rejection record. No candidate, no projection, no
   asset, no partial anything.
3. **Not promotion.** Nothing Apply writes leaves the disposable session. The
   promotion receipt, the reviewed digest constant, the authored members and
   every generated projection are byte-identical afterwards, and the promoted
   path still passes its own `--check`.

These run against the REAL repository, because the compiler locates its own root
from its manifest and reads the accepted master from there — pointing it at a
copied tree would prove something about a copy. Every session they open is
deleted afterwards, and the tracked tree is compared before and after.
"""

from __future__ import annotations

import contextlib
import hashlib
import io
import json
import os
import shutil
import subprocess
import sys
import unittest
from pathlib import Path
from unittest import mock

from workbench_test_support import REPO_ROOT, TOOLS
from boundary_test_support import BoundaryTestCase

if str(TOOLS) not in sys.path:
    sys.path.insert(0, str(TOOLS))

from workbench import apply as apply_module  # noqa: E402
from workbench import bridge  # noqa: E402
from workbench import imageops  # noqa: E402
from workbench_integrity import carried_tree  # noqa: E402
from workbench import operations as operation_log  # noqa: E402
from workbench import replay as replay_module  # noqa: E402
from workbench.packet import now  # noqa: E402
from workbench.projection import (  # noqa: E402
    DEFAULT_PROJECTION_PATH,
    StaleSelection,
    WorkbenchError,
    load,
)
from workbench.session import APPLY_DIR, Session, open_session  # noqa: E402

#: The land this suite edits. The Workbench addresses a land by name at every
#: compiler entry point, and the authoring fixture is the one that exists to be
#: edited in a test.
FIXTURE_LAND = "authoring_fixture"

MASTER = "content/authoring-fixture/fixture-surface.tmj"
RECEIPT = "content/authoring-fixture/promotion.json"
DIGEST_CONSTANT = "crates/tme-authoring/src/contract/fixture.rs"
ASSET = "content/authoring-fixture/fixture-swatch.png"
PROVENANCE = "content/authoring-fixture/asset-provenance.json"

#: An edit the compiler accepts, and one it does not. Both move the same
#: landmark, so the difference between an Apply and a rejection is one cell.
GOOD_MOVE = {"landmark_id": "fixture_ruin_marker", "to": {"x": 6, "y": 11}}
BAD_MOVE = {"landmark_id": "fixture_ruin_marker", "to": {"x": 0, "y": 0}}


def digest(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


class CarriedSourceIntegrity(BoundaryTestCase):
    """Break scratch sources and local output independently of real Apply."""

    def test_ignored_output_and_nested_repositories_do_not_change_snapshot(self) -> None:
        self.repo.write(".gitignore", "/web/node_modules/\n/target/\n*.py[cod]\n")
        self.repo.write("render/.gitignore", ".cache/\n")
        self.repo.write("source.py", "accepted\n")
        self.repo.track(".gitignore", "render/.gitignore", "source.py")
        before = carried_tree(self.repo.path)
        for name in ("web/node_modules/pkg/index.js", "target/debug/build",
                     "render/.cache/cache", "nested/cache.pyc"):
            self.repo.write(name, "disposable\n")
        nested = self.repo.path / "agents/nested"
        nested.mkdir(parents=True)
        subprocess.run(["git", "init", "-q", str(nested)], check=True)
        (nested / "source.py").write_text("another repository\n")
        # A linked worktree has a .git file, not a directory. Both are boundaries.
        identity = {f"GIT_{role}_{field}": value
                    for role in ("AUTHOR", "COMMITTER")
                    for field, value in (("NAME", "Fixture"), ("EMAIL", "fixture@example.invalid"))}
        subprocess.run(["git", "-C", str(self.repo.path), "commit", "-qm", "fixture"],
                       env={**os.environ, **identity}, check=True, capture_output=True)
        self.repo._git("worktree", "add", "--detach", "agents/linked", "HEAD")
        self.repo.write("agents/linked/web/node_modules/pkg/index.js", "unrelated\n")
        self.assertEqual(carried_tree(self.repo.path), before)

    def test_nested_negation_and_force_tracked_ignored_files_remain_protected(self) -> None:
        self.repo.write(".gitignore", "*.cache\n")
        self.repo.write("nested/.gitignore", "*.txt\n!keep.txt\n")
        self.repo.write("nested/hidden.txt", "ignored\n")
        self.repo.write("nested/keep.txt", "untracked source\n")
        self.repo.write("source.cache", "explicitly carried\n")
        self.repo._git("add", "-f", "source.cache")
        snapshot = carried_tree(self.repo.path)
        self.assertIn("nested/keep.txt", snapshot)
        self.assertIn("source.cache", snapshot)
        self.assertNotIn("nested/hidden.txt", snapshot)

    def test_source_edits_additions_and_deletions_change_snapshot(self) -> None:
        self.repo.write("tracked.py", "original\n")
        self.repo.track("tracked.py")
        self.repo.write("untracked.py", "original\n")
        for name in ("tracked.py", "untracked.py"):
            with self.subTest(name=name):
                before = carried_tree(self.repo.path)
                path = self.repo.write(name, "mutated\n")
                self.assertNotEqual(carried_tree(self.repo.path), before)
                before = carried_tree(self.repo.path)
                path.unlink()
                self.assertNotEqual(carried_tree(self.repo.path), before)
        before = carried_tree(self.repo.path)
        self.repo.write("new.py", "stray source write\n")
        self.assertNotEqual(carried_tree(self.repo.path), before)

    def test_clean_copy_uses_the_same_inventory_after_its_fresh_git_init(self) -> None:
        from run_clean_clone_proof import populate
        from boundary_test_support import TempRepo

        self.repo.write(".gitignore", "/web/node_modules/\n")
        self.repo.write("tracked.py", "accepted\n")
        self.repo.track(".gitignore", "tracked.py")
        self.repo.write("untracked.py", "under review\n")
        self.repo.write("web/node_modules/pkg/index.js", "dependency\n")
        copied = TempRepo()
        self.addCleanup(copied.cleanup)
        populate(copied.path, self.repo.path)
        copied._git("add", "-A")
        self.assertEqual(carried_tree(copied.path), carried_tree(self.repo.path))

    def test_missing_git_metadata_fails_instead_of_claiming_unchanged(self) -> None:
        from boundary_common import ConfigError

        shutil.rmtree(self.repo.path / ".git")
        with self.assertRaises(ConfigError):
            carried_tree(self.repo.path)

    def test_demo_rejects_a_source_mutation_instead_of_only_printing_it(self) -> None:
        import workbench_demo

        self.repo.write("untracked.py", "before\n")

        def mutate_source(*_args):
            self.repo.write("untracked.py", "after\n")
            return {"apply_id": "apply-test", "path": "rejection.json", "record": {
                "stage": "validate", "assertion": "rejected", "operation": {
                    "record_id": "operation-test", "verb": "move_landmark"}}}

        with mock.patch.object(workbench_demo, "ROOT", self.repo.path), \
                mock.patch.object(workbench_demo, "stage", return_value={"record_id": "operation-test"}), \
                mock.patch.object(workbench_demo, "post", side_effect=mutate_source), \
                contextlib.redirect_stdout(io.StringIO()):
            with self.assertRaisesRegex(RuntimeError, "changed carried files: untracked.py"):
                workbench_demo.rejection_act("http://unused.invalid", "selection-test")


class V1Session(unittest.TestCase):
    """One real session in this repository, opened and then removed."""

    def setUp(self) -> None:
        super().setUp()
        self.projection = load(REPO_ROOT, DEFAULT_PROJECTION_PATH)
        self.session = open_session(self.projection, f"session-test-{id(self):x}")
        self.addCleanup(shutil.rmtree, self.session.directory, ignore_errors=True)
        self.selection = self.point(6, 11)

    def point(self, x: int, y: int) -> str:
        packet = self.session.record_logical_selection(
            self.projection,
            {"member": "surface", "gesture": "click", "cell": {"x": x, "y": y}},
        )
        return packet["selection_id"]

    def stage(self, verb: str, parameters: dict, **overrides) -> dict:
        body = {
            "record_id": self.session.next_record_id(),
            "recorded_at": now(),
            "author": "owner",
            "selection_id": self.selection,
            "operation_class": operation_log.CLASS_TRUTH,
            "member": "surface",
            "editable_member": self.projection.candidate_member,
            "verb": verb,
            "parameters": parameters,
        }
        body.update(overrides)
        return self.session.stage_operation(operation_log.build(**body))

    def stage_asset(self, *, mask_rect=(10, 10, 6, 4), colour=(26, 26, 26, 255)) -> dict:
        provenance = json.loads((REPO_ROOT / PROVENANCE).read_text())["assets"][0]
        x, y, width, height = mask_rect
        payload = imageops.write_mask(
            [(x + dx, y + dy) for dy in range(height) for dx in range(width)]
        )
        mask = self.session.write_artifact("commit/commit-test.pbm", payload)
        return self.stage(
            "edit_region",
            {
                "source": {"path": provenance["path"], "sha256": provenance["sha256"]},
                "commit_mask": mask,
                "context": {"margin": 3},
            },
            operation_class=operation_log.CLASS_ASSET,
            member="asset",
            adapter={"adapter": "palette_fill", "parameters": {"colour": list(colour)}},
        )

    def apply(self):
        return apply_module.apply(self.session)

    def session_files(self) -> set[str]:
        return {
            str(path.relative_to(self.session.directory))
            for path in self.session.directory.rglob("*")
            if path.is_file()
        }


class ReplayIsDeterministic(V1Session):
    def test_an_empty_log_reproduces_the_accepted_master_byte_for_byte(self) -> None:
        """Kills a replay that reformats, reorders, or drops a field in passing.

        Nothing is staged, so the candidate must BE the master. If this fails,
        every candidate differs from its base in ways nobody asked for, and the
        diff an owner reviews is noise around the edit.
        """
        outcome = replay_module.preview(self.session)
        self.assertTrue(outcome.accepted, outcome.detail)
        self.assertEqual(
            (REPO_ROOT / outcome.candidate["path"]).read_bytes(),
            (REPO_ROOT / MASTER).read_bytes(),
        )
        self.assertEqual(outcome.candidate["sha256"], digest(REPO_ROOT / MASTER))

    def test_the_same_log_twice_produces_the_same_bytes(self) -> None:
        """Kills a replay carrying a timestamp, a run id, or a hash-ordered map."""
        self.stage("move_landmark", GOOD_MOVE)
        first = replay_module.preview(self.session)
        second = replay_module.preview(self.session)
        self.assertTrue(first.accepted and second.accepted)
        self.assertEqual(first.candidate["sha256"], second.candidate["sha256"])
        self.assertNotEqual(first.candidate["sha256"], digest(REPO_ROOT / MASTER))

    def test_a_retracted_operation_leaves_no_trace_in_the_candidate(self) -> None:
        """Kills a replay that reads the log instead of the effective set."""
        record = self.stage("move_landmark", GOOD_MOVE)
        edited = replay_module.preview(self.session).candidate["sha256"]
        self.session.retract_operation(record["record_id"], "wrong cell")
        self.assertEqual(
            replay_module.preview(self.session).candidate["sha256"],
            digest(REPO_ROOT / MASTER),
        )
        self.assertNotEqual(edited, digest(REPO_ROOT / MASTER))


class ApplyIsAtomic(V1Session):
    def test_a_rejected_apply_leaves_the_carried_tree_byte_identical(self) -> None:
        """The whole tree, file by file — the claim Apply makes about failure."""
        before = carried_tree(REPO_ROOT)
        self.stage("move_landmark", BAD_MOVE)
        applied = self.apply()
        self.assertFalse(applied.accepted)
        self.assertEqual(carried_tree(REPO_ROOT), before)

    def test_a_rejected_apply_writes_the_rejection_record_and_nothing_else(self) -> None:
        """Kills a half-written candidate somebody could mistake for an outcome.

        Everything is built in a pending directory and only becomes visible in
        one rename. A rejection removes it, so the session gains exactly one file
        and no candidate exists anywhere.
        """
        before = self.session_files()
        self.stage("move_landmark", BAD_MOVE)
        applied = self.apply()
        added = self.session_files() - before
        self.assertEqual(
            added,
            {f"{APPLY_DIR}/{applied.apply_id}.rejection.json"},
            "a rejected Apply wrote something besides its rejection record",
        )
        self.assertEqual(list(self.session.directory.glob(f"{APPLY_DIR}/.pending-*")), [])

    def test_a_rejection_names_the_stage_the_operation_and_the_validators_words(self) -> None:
        """Kills a rejection an owner cannot act on."""
        self.stage("move_landmark", BAD_MOVE)
        record = self.apply().record
        self.assertEqual(record["stage"], "validate")
        self.assertEqual(record["operation"]["verb"], "move_landmark")
        self.assertEqual(
            record["assertion"], "landmark fixture_ruin_marker stands on a blocked cell"
        )

    def test_a_failing_asset_edit_takes_the_whole_apply_with_it(self) -> None:
        """Kills "apply the ones that passed".

        The map edit here is perfectly good. The asset edit names a mask whose
        digest has moved, and the Apply that carries both writes nothing at all.
        """
        before = self.session_files()
        self.stage("move_landmark", GOOD_MOVE)
        self.stage_asset()
        (self.session.directory / "commit/commit-test.pbm").write_bytes(
            imageops.write_mask([(10, 10), (11, 10)])
        )
        applied = self.apply()
        self.assertFalse(applied.accepted)
        self.assertEqual(applied.record["stage"], "asset")
        added = self.session_files() - before
        self.assertEqual(
            {name for name in added if not name.startswith("commit/")},
            {f"{APPLY_DIR}/{applied.apply_id}.rejection.json"},
        )

    def test_an_accepted_apply_writes_every_output_its_receipt_names(self) -> None:
        """Kills a receipt that describes files that are not there."""
        self.stage("move_landmark", GOOD_MOVE)
        self.stage_asset()
        applied = self.apply()
        self.assertTrue(applied.accepted, applied.record)
        roles = [output["role"] for output in applied.record["outputs"]]
        self.assertEqual(
            roles, ["candidate_master", "candidate_projection", "candidate_asset"]
        )
        for output in applied.record["outputs"]:
            path = REPO_ROOT / output["path"]
            self.assertTrue(path.is_file(), f"{output['path']} is not there")
            self.assertEqual(digest(path), output["sha256"])

    def test_the_receipt_records_what_the_adapter_did_outside_the_mask(self) -> None:
        """The preservation rule's provenance half, on the record.

        `palette_fill` fills everything it is handed and knows nothing about the
        commit mask — which is the point. The edit stands, exactly the mask's
        pixels changed, and what the adapter painted outside them is recorded as
        discarded rather than quietly forgotten.
        """
        self.stage_asset()
        applied = self.apply()
        self.assertTrue(applied.accepted, applied.record)
        edit = applied.record["asset_edits"][0]
        self.assertEqual(edit["changed_pixels"], 24)
        self.assertGreater(edit["adapter_wrote_outside_the_mask"]["pixels"], 0)
        source = imageops.decode((REPO_ROOT / ASSET).read_bytes())
        result = imageops.decode(
            (REPO_ROOT / applied.record["outputs"][-1]["path"]).read_bytes()
        )
        mask = imageops.read_mask(
            (self.session.directory / "commit/commit-test.pbm").read_bytes(),
            image_width=source.width,
            image_height=source.height,
        )
        self.assertTrue(imageops.preserved_outside(source, result, mask))


class ApplyIsNotPromotion(V1Session):
    """Spec section 8.1, proven rather than promised."""

    def test_the_promotion_anchors_are_byte_identical_after_a_full_loop(self) -> None:
        """The rule that matters most: neither anchor moves, ever, from here.

        The receipt on disk and the reviewed digest constant in Rust source are
        the double anchor. This runs a complete V1 loop — point, stage a map
        edit, stage a visual edit, preview, apply, accept — and then compares
        both anchors, both authored members, and every generated projection.
        """
        watched = [MASTER, RECEIPT, DIGEST_CONSTANT, ASSET, PROVENANCE]
        watched += [
            str(path.relative_to(REPO_ROOT))
            for path in (REPO_ROOT / "content/authoring-fixture/generated").iterdir()
        ]
        before = {name: digest(REPO_ROOT / name) for name in watched}

        self.stage("move_landmark", GOOD_MOVE)
        self.stage_asset()
        replay_module.preview(self.session)
        applied = self.apply()
        self.assertTrue(applied.accepted, applied.record)
        self.session.record_candidate_acceptance(
            candidate_sha256=applied.record["outputs"][0]["sha256"],
            apply_id=applied.apply_id,
            note="the marker sits right now",
        )

        self.assertEqual({name: digest(REPO_ROOT / name) for name in watched}, before)

    def test_every_output_lands_inside_the_session(self) -> None:
        """Kills an Apply that writes anywhere a build could read from."""
        self.stage("move_landmark", GOOD_MOVE)
        self.stage_asset()
        applied = self.apply()
        session = self.session.directory.resolve()
        for output in applied.record["outputs"]:
            resolved = (REPO_ROOT / output["path"]).resolve()
            self.assertIn(session, resolved.parents, f"{output['path']} escaped the session")

    def test_the_session_refuses_to_address_anything_outside_itself(self) -> None:
        """THE MUTANT for the write guard.

        Every V1 artifact path is made by `Session.artifact`. Plant a caller that
        tries to write tracked content through it and watch it refuse — that is
        what makes "the Workbench cannot write tracked content" a property of the
        code rather than a property of every caller being careful.
        """
        for escape in ("../../../content/authoring-fixture/promotion.json", "/etc/passwd"):
            with self.subTest(path=escape):
                with self.assertRaises(WorkbenchError) as caught:
                    self.session.artifact(escape)
                self.assertIn("outside the session directory", str(caught.exception))
        self.assertTrue(
            str(self.session.artifact("apply", "apply-0001")).startswith(
                str(self.session.directory.resolve())
            )
        )

    def test_the_promoted_path_still_passes_its_own_check_afterwards(self) -> None:
        """Kills anything leaking from a candidate into the promoted projection.

        `--check` asserts the tracked projections are exactly what a fresh
        compile of the attested master writes. Running it after an Apply is the
        end-to-end form of "no candidate ever becomes an input to the promoted
        load path".
        """
        self.stage("move_landmark", GOOD_MOVE)
        self.assertTrue(self.apply().accepted)
        finished = subprocess.run(
            ["cargo", "run", "--quiet", "--locked", "-p", "tme-authoring", "--", "--check"],
            cwd=REPO_ROOT,
            capture_output=True,
            text=True,
        )
        self.assertEqual(finished.returncode, 0, finished.stderr)

    def test_the_candidate_report_still_cannot_become_a_surface(self) -> None:
        """The type-level proof is a doctest; this guards its existence.

        The proof itself runs in the Rust lane — a `compile_fail` doctest on
        `validate_surface_candidate`. What this catches is somebody deleting the
        fence and leaving a docstring that only claims the property.
        """
        source = (REPO_ROOT / "crates/tme-authoring/src/candidate.rs").read_text()
        self.assertIn("```compile_fail", source)
        self.assertIn("takes_member(report.into());", source)


class StalenessStopsAnApplyBeforeItComputesAnything(V1Session):
    def test_a_packet_bound_to_bytes_that_moved_stops_the_apply(self) -> None:
        """Kills an Apply that edits a world the selection never saw.

        The packet is edited to claim a digest the master does not carry, which
        is exactly what a real moved master looks like from the packet's side.
        Nothing is replayed, nothing is validated, and the rejection names the
        file whose digest moved.
        """
        self.stage("move_landmark", GOOD_MOVE)
        path = self.session.directory / "selections" / f"{self.selection}.json"
        packet = json.loads(path.read_bytes())
        for record in packet["source"]["digests"]:
            if record["role"] == "master":
                record["sha256"] = "0" * 64
        path.write_text(json.dumps(packet, indent=2) + "\n")

        applied = self.apply()
        self.assertFalse(applied.accepted)
        self.assertEqual(applied.record["stage"], "staleness")
        self.assertIn(MASTER, applied.record["assertion"])
        self.assertIsNone(applied.record["detail"]["candidate"])

    def test_the_bridge_refuses_a_base_it_was_not_told_to_expect(self) -> None:
        """Kills a replay against bytes the caller never looked at."""
        directory = self.session.artifact("preview")
        directory.mkdir(parents=True, exist_ok=True)
        payload = directory / "operations.json"
        payload.write_text(json.dumps(operation_log.truth_operation_set([])) + "\n")
        answer = bridge.replay(
            REPO_ROOT,
            land=FIXTURE_LAND,
            operations=payload,
            output_directory=directory,
            expect_base_sha256="0" * 64,
        )
        self.assertFalse(answer.yes)
        self.assertEqual(answer.document["stage"], "base")
        self.assertIn("the caller expected", answer.document["error"])

    def test_verify_is_what_stops_it(self) -> None:
        """The refusal type, so a caller can tell staleness from every other no."""
        with self.assertRaises(StaleSelection):
            replay_module.verify(REPO_ROOT, [replay_module.Source("master", MASTER, "0" * 64)])


class AgentParityHoldsForTheWholeV1Loop(V1Session):
    def test_an_operation_staged_with_a_text_editor_applies_identically(self) -> None:
        """The parity law at its strongest: no tool at all on the agent's side.

        One JSON line appended to the log by hand — no CLI, no browser, no import
        — produces exactly the candidate the same operation produces when the
        Workbench stages it. If this ever fails, "an agent can do everything the
        owner can" has quietly become a claim about a code path nobody uses.
        """
        self.stage("move_landmark", GOOD_MOVE)
        through_the_tool = replay_module.preview(self.session).candidate["sha256"]

        by_hand = open_session(self.projection, f"session-test-hand-{id(self):x}")
        self.addCleanup(shutil.rmtree, by_hand.directory, ignore_errors=True)
        shutil.copytree(
            self.session.directory / "selections",
            by_hand.directory / "selections",
            dirs_exist_ok=True,
        )
        line = {
            "schema_version": 1,
            "kind": "operation_staged",
            "record_id": "op-0001",
            "recorded_at": "2026-08-20T00:00:00Z",
            "author": "agent",
            "selection_id": self.selection,
            "operation": {
                "class": "truth",
                "member": "surface",
                "verb": "move_landmark",
                "parameters": GOOD_MOVE,
                "adapter": None,
            },
            "comment": "written with a text editor",
        }
        with (by_hand.directory / "operations.jsonl").open("a", encoding="utf-8") as stream:
            stream.write(json.dumps(line) + "\n")

        self.assertEqual(
            replay_module.preview(by_hand).candidate["sha256"], through_the_tool
        )

    def test_the_command_line_applies_what_the_module_staged(self) -> None:
        """Kills a CLI that is a second implementation rather than a caller."""
        self.stage("move_landmark", GOOD_MOVE)
        listed = subprocess.run(
            [
                sys.executable,
                str(TOOLS / "workbench" / "stage.py"),
                "list",
                "--session",
                str(self.session.directory),
                "--json",
            ],
            cwd=REPO_ROOT,
            capture_output=True,
            text=True,
        )
        self.assertEqual(listed.returncode, 0, listed.stderr)
        self.assertEqual(
            json.loads(listed.stdout), operation_log.summary(self.session.staged())
        )

        applied = subprocess.run(
            [
                sys.executable,
                str(TOOLS / "workbench" / "apply.py"),
                str(self.session.directory),
                "--json",
            ],
            cwd=REPO_ROOT,
            capture_output=True,
            text=True,
        )
        self.assertEqual(applied.returncode, 0, applied.stderr)
        receipt = json.loads(applied.stdout)
        self.assertEqual(receipt["kind"], "workbench_apply_receipt")
        self.assertEqual(receipt["grants"]["promotion"], False)


class TheVocabularyIsOneTable(unittest.TestCase):
    def test_every_published_verb_carries_the_rejection_it_can_trip(self) -> None:
        """Spec section 6.3's second binding constraint, read off the table.

        Every verb must have a validator failure it provably triggers. The proof
        is in the Rust corpus; what this asserts is that the published table says
        which one, so an owner staging an edit can see what it will be refused
        for before they stage it.
        """
        vocabulary = bridge.describe_operations(REPO_ROOT, FIXTURE_LAND)
        self.assertEqual(vocabulary["class"], "truth")
        verbs = {spec["verb"]: spec for spec in vocabulary["verbs"]}
        self.assertEqual(
            sorted(verbs),
            [
                "move_landmark",
                "move_structure",
                "set_route",
                "set_structure_access",
                "set_terrain",
                "set_transition_endpoint",
            ],
        )
        for name, spec in verbs.items():
            with self.subTest(verb=name):
                self.assertTrue(spec["summary"].strip())
                self.assertTrue(spec["rejects"].strip())
                self.assertTrue(spec["parameters"])

    def test_a_closed_parameter_carries_its_own_choices(self) -> None:
        """Kills an interface that has to read prose to build an input."""
        vocabulary = bridge.describe_operations(REPO_ROOT, FIXTURE_LAND)
        terrain = next(spec for spec in vocabulary["verbs"] if spec["verb"] == "set_terrain")
        choices = {row["name"]: row["choices"] for row in terrain["parameters"]}
        self.assertIsNone(choices["cells"])
        self.assertEqual(choices["class"], vocabulary["terrain_classes"])


if __name__ == "__main__":
    unittest.main()
