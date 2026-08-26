"""The staged-operation log: its envelope, its order, and what it refuses.

The log is the shared language between an owner and an agent, so its shape is
worth proving on its own, before anything replays it. Every test here is a
refusal or an ordering claim; nothing here runs the compiler.

The division of ownership this file holds to: the ENVELOPE is the Workbench's —
who staged it, which selection it derives from, which class it belongs to — and
the PARAMETERS are the vocabulary owner's. So there is no test here asserting
what `move_landmark` takes, and there should not be one: a second statement of
the parameter shapes is the drift this split exists to prevent.
"""

from __future__ import annotations

import sys
import unittest
from pathlib import Path

from workbench_test_support import TOOLS

if str(TOOLS) not in sys.path:
    sys.path.insert(0, str(TOOLS))

from workbench import operations  # noqa: E402


def staged(record_id: str, **overrides) -> dict:
    body = {
        "record_id": record_id,
        "recorded_at": "2026-08-20T00:00:00Z",
        "author": "owner",
        "selection_id": "sel-0001",
        "operation_class": operations.CLASS_TRUTH,
        "member": "surface",
        "editable_member": "surface",
        "verb": "set_terrain",
        "parameters": {"cells": [], "class": "testland_grass"},
    }
    body.update(overrides)
    return operations.build(**body)


class TheEnvelopeIsRefusedWhenItIsNotOne(unittest.TestCase):
    def test_a_record_with_no_selection_is_refused(self) -> None:
        """Kills an operation that is not an act upon an address.

        Without a packet there is no bound digest set to re-verify at Apply, and
        the tool's one product requirement — exact pointing — degrades into an
        agent asserting coordinates it typed.
        """
        with self.assertRaises(operations.OperationRefused) as caught:
            staged("op-0001", selection_id="")
        self.assertIn("selection_id", str(caught.exception))

    def test_an_unknown_class_is_refused_naming_the_classes(self) -> None:
        """Kills a fourth operation class arriving without a decision behind it."""
        with self.assertRaises(operations.OperationRefused) as caught:
            staged("op-0001", operation_class="tuning")
        for name in operations.CLASSES:
            self.assertIn(name, str(caught.exception))

    def test_a_dressing_operation_is_refused_with_the_ruling(self) -> None:
        """The dressing ruling, enforced rather than documented.

        The class exists in the vocabulary and ships zero verbs, because this
        project has accepted no authored presentation fact and there is nothing
        to dress. Staging one is refused with the reason, not with a shrug.
        """
        with self.assertRaises(operations.OperationRefused) as caught:
            staged("op-0001", operation_class=operations.CLASS_DRESSING)
        self.assertEqual(str(caught.exception), operations.DRESSING_RULING)
        self.assertIn("nothing to dress", operations.DRESSING_RULING)

    def test_a_truth_operation_against_another_member_is_refused(self) -> None:
        """Kills an edit aimed at a member with no candidate entry point."""
        with self.assertRaises(operations.OperationRefused) as caught:
            staged("op-0001", member="interior")
        self.assertIn("no candidate entry point", str(caught.exception))

    def test_parameters_and_the_adapter_block_must_be_objects(self) -> None:
        """Kills a record whose payload the compiler could only guess at."""
        with self.assertRaises(operations.OperationRefused):
            staged("op-0001", parameters=[])
        with self.assertRaises(operations.OperationRefused):
            staged("op-0001", adapter="palette_fill")

    def test_an_owner_gesture_and_an_agent_proposal_are_the_same_record(self) -> None:
        """The agent-parity law, in the only place it can be asserted structurally."""
        owner = staged("op-0001", author="owner")
        agent = staged("op-0001", author="agent")
        self.assertNotEqual(owner["author"], agent["author"])
        owner.pop("author")
        agent.pop("author")
        self.assertEqual(owner, agent)


class TheEffectiveSetIsADerivation(unittest.TestCase):
    def test_the_effective_set_is_the_log_in_log_order(self) -> None:
        """Kills a resolver that reorders, deduplicates, or sorts.

        Replay is the log, in the order the log has: two edits to one cell mean
        the later one wins, and any reordering changes what Apply produces.
        """
        records = [staged(f"op-{index:04d}") for index in range(1, 4)]
        self.assertEqual(
            [record["record_id"] for record in operations.effective(records, editable_member="surface")],
            ["op-0001", "op-0002", "op-0003"],
        )

    def test_v0_records_pass_through_untouched(self) -> None:
        """Kills a resolver that trips over the log V0 already wrote.

        One file holds both, so the resolver reads a selection record and a
        comment as what they are: not staged operations, and not errors either.
        """
        log = [
            {"kind": "selection_recorded", "record_id": "op-0001", "operation": None},
            {"kind": "owner_comment", "record_id": "op-0002", "operation": None},
            staged("op-0003"),
        ]
        self.assertEqual(len(operations.effective(log, editable_member="surface")), 1)

    def test_a_retraction_removes_the_operation_it_names(self) -> None:
        """Kills a retraction that deletes a line instead of appending one.

        What was tried and dropped is part of what happened in a session, so the
        log keeps it and the derivation drops it.
        """
        log = [
            staged("op-0001"),
            staged("op-0002"),
            operations.retraction(
                record_id="op-0003",
                recorded_at="2026-08-20T00:00:01Z",
                author="owner",
                retracts="op-0001",
                reason="wrong cell",
            ),
        ]
        self.assertEqual(
            [record["record_id"] for record in operations.effective(log, editable_member="surface")], ["op-0002"]
        )
        self.assertEqual(len(log), 3, "the retracted record is still in the log")

    def test_a_retraction_naming_nothing_is_refused(self) -> None:
        """Kills a log that quietly ignores an instruction it cannot carry out."""
        log = [
            staged("op-0001"),
            operations.retraction(
                record_id="op-0002",
                recorded_at="2026-08-20T00:00:01Z",
                author="owner",
                retracts="op-0099",
                reason="",
            ),
        ]
        with self.assertRaises(operations.OperationRefused) as caught:
            operations.effective(log, editable_member="surface")
        self.assertIn("never staged", str(caught.exception))

    def test_retracting_twice_is_refused(self) -> None:
        """Kills a log whose second retraction reads as a no-op."""
        retract = operations.retraction(
            record_id="op-0003",
            recorded_at="2026-08-20T00:00:01Z",
            author="owner",
            retracts="op-0001",
            reason="",
        )
        second = dict(retract, record_id="op-0004")
        with self.assertRaises(operations.OperationRefused) as caught:
            operations.effective(
                [staged("op-0001"), retract, second], editable_member="surface"
            )
        self.assertIn("already retracted", str(caught.exception))

    def test_a_duplicated_record_id_is_refused(self) -> None:
        """Kills an ambiguous retraction target."""
        with self.assertRaises(operations.OperationRefused) as caught:
            operations.effective(
                [staged("op-0001"), staged("op-0001")], editable_member="surface"
            )
        self.assertIn("twice", str(caught.exception))


class TheCompilerReadsOnlyWhatItOwns(unittest.TestCase):
    def test_the_truth_set_carries_the_fields_the_compiler_declares(self) -> None:
        """Kills a payload the compiler would refuse for an unknown field.

        The compiler's own `StagedOperation` denies unknown fields, so this shape
        is checked on the other side of the bridge as well as here — which is the
        point: the envelope is described in one place and enforced in two.
        """
        payload = operations.truth_operation_set([staged("op-0001")])
        self.assertEqual(payload["kind"], "workbench_truth_operation_set")
        self.assertEqual(
            sorted(payload["operations"][0]),
            ["author", "class", "member", "parameters", "record_id", "verb"],
        )

    def test_asset_operations_are_not_sent_to_the_compiler(self) -> None:
        """Kills a routing mistake that would hand a picture to a map validator."""
        log = [
            staged("op-0001"),
            staged("op-0002", operation_class=operations.CLASS_ASSET, member="asset",
                   verb="edit_region", parameters={"source": {}}),
        ]
        payload = operations.truth_operation_set(operations.effective(log, editable_member="surface"))
        self.assertEqual([row["record_id"] for row in payload["operations"]], ["op-0001"])


if __name__ == "__main__":
    unittest.main()
