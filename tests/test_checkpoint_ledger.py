"""The read-only checkpoint ledger the browser journeys assert against.

`web/proof/death-return-proof.mjs` and the expedition proof's death journey ask
their runner for an authoritative ledger before the death boundary and for an
audit after the return. The reviewed browser oracle only compared counts — an
empty inventory and an empty purse passed, and so did deleting an item,
substituting one for another, or duplicating an instance.

These cases hold the runner's half: the reader resolves every instance to the
collections the stored checkpoint actually has and refuses an instance that is
in none of them or in more than one, and the audit fails on loss, creation,
identity replacement, quantity change and currency minted or burned. The typed
rules owner for the same boundary is
`crates/tme-rules/tests/first_expedition_death_ledger.rs`; this is the durable
copy the browser half reads.
"""

import json
import sys
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / "tools"))

from run_death_return_proof import (  # noqa: E402
    LedgerAudit,
    audit_ledgers,
    checkpoint_ledger,
    ledger_movement,
)

POSITION = {"realm": "first_expedition", "level": "d1_entry", "position": {"x": 23, "y": 9}}


def checkpoint(*, carried_items=None, carried_gold=100, ground=(), corpses=None,
               ground_gold=None, banks=None, instances=None, scavenger_items=None):
    """A stored checkpoint in the shape `FacetCheckpointPayloadV1` serialises.

    Every item instance that is placed somewhere is declared in
    `item_instances` with a derived definition, so a case only has to say where
    things are; `instances` adds extra definitions, which is how the
    no-collection and duplicate cases are built.
    """
    carried_items = carried_items or {}
    scavenger_items = scavenger_items or {}
    corpses = corpses or {}
    items = {}
    for item_instance_id in (
        list(carried_items.values())
        + list(scavenger_items.values())
        + [row["item_instance_id"] for row in ground]
        + [item for corpse in corpses.values() for item in (corpse.get("contents") or {}).values()]
    ):
        items[item_instance_id] = {"definition_id": f"item/{item_instance_id}", "quantity": 1}
    items.update(instances or {})
    world = {
        "actors": [
            {
                "id": "created/proof/c",
                "character_id": "proof/c",
                "carried": {"items": carried_items, "gold": {"left_hand": 0, "right_hand": 0, "sack": carried_gold}},
            },
            {
                "id": "cellar_scavenger",
                "carried": {"items": scavenger_items, "gold": {"left_hand": 0, "right_hand": 0, "sack": 0}},
            },
        ],
        "item_instances": items,
        "ground_items": list(ground),
        "corpses": corpses,
        "merchant_inventories": [],
        "locker_vaults": {},
        "item_offers": {},
        "ground_gold": ground_gold or {},
        "banks": banks or {},
    }
    return {"world": world}


class ReadingTheLedger(unittest.TestCase):
    def test_every_instance_resolves_to_exactly_one_place_and_every_coin_is_counted(self):
        ledger = checkpoint_ledger(
            checkpoint(
                carried_items={"right_hand": "weathered_staff"},
                carried_gold=100,
                corpses={
                    "corpse:1": {
                        "contents": {"sack_item_1": "wizard_spell_book"},
                        "gold": 25,
                    }
                },
                ground_gold={"gold:1": {"amount": 5}},
                banks={"bank/town_bank/first_expedition": {"balances": {"proof/c": 40}}},
                instances={
                    "wizard_spell_book": {"definition_id": "item/wizard_spell_book", "quantity": 1},
                },
            )
        )
        self.assertEqual(ledger["unowned"], [])
        self.assertEqual(ledger["item_count"], 2)
        self.assertEqual(
            ledger["items"]["weathered_staff"]["locations"],
            ["carried:proof/c:right_hand"],
        )
        self.assertEqual(
            ledger["items"]["wizard_spell_book"]["locations"], ["corpse:corpse:1:sack_item_1"]
        )
        self.assertEqual(
            ledger["gold"],
            {
                "actor:proof/c:sack": 100,
                "corpse:corpse:1": 25,
                "ground:gold:1": 5,
                "bank:bank/town_bank/first_expedition:proof/c": 40,
            },
        )
        self.assertEqual(ledger["gold_total"], 170)

    def test_an_instance_in_no_collection_is_reported_rather_than_counted(self):
        ledger = checkpoint_ledger(
            checkpoint(instances={"orphan": {"definition_id": "item/orphan", "quantity": 1}})
        )
        self.assertEqual(ledger["unowned"], ["orphan"])

    def test_an_instance_in_two_collections_is_reported_rather_than_double_counted(self):
        ledger = checkpoint_ledger(
            checkpoint(
                carried_items={"right_hand": "weathered_staff"},
                corpses={"corpse:1": {"contents": {"right_hand": "weathered_staff"}, "gold": 0}},
            )
        )
        self.assertEqual(ledger["unowned"], ["weathered_staff"])


class AuditingTwoLedgers(unittest.TestCase):
    def before(self):
        return checkpoint_ledger(
            checkpoint(
                carried_items={"right_hand": "weathered_staff"},
                carried_gold=100,
                instances={"wizard_spell_book": {"definition_id": "item/wizard_spell_book", "quantity": 1}},
                corpses={"corpse:1": {"contents": {"sack_item_1": "wizard_spell_book"}, "gold": 0}},
            )
        )

    def test_a_legitimate_theft_is_a_move_and_not_a_defect(self):
        # The monster took the staff the character dropped. Fewer items are
        # visible from the return destination, and nothing was created.
        after = checkpoint_ledger(
            checkpoint(
                carried_items={},
                carried_gold=0,
                corpses={"corpse:1": {"contents": {"sack_item_1": "wizard_spell_book"},
                                      "gold": 100}},
                scavenger_items={"right_hand": "weathered_staff"},
            )
        )
        self.assertEqual(after["unowned"], [])
        self.assertEqual(after["items"]["weathered_staff"]["locations"],
                         ["carried:cellar_scavenger:right_hand"])
        self.assertEqual(after["items"]["wizard_spell_book"]["locations"],
                         ["corpse:corpse:1:sack_item_1"])
        self.assertEqual(audit_ledgers(self.before(), after), [])
        self.assertIn("weathered_staff", ledger_movement(self.before(), after))

    def test_the_empty_world_that_passed_the_count_oracle_fails_here(self):
        empty = {"items": {}, "gold": {}, "item_count": 0, "gold_total": 0, "unowned": []}
        defects = audit_ledgers(self.before(), empty)
        self.assertTrue(any("stopped existing" in defect for defect in defects), defects)
        self.assertTrue(any("created or destroyed" in defect for defect in defects), defects)

    def test_a_same_count_substitution_fails(self):
        after = self.before()
        after["items"]["weathered_staff"]["definition_id"] = "item/bright_staff"
        defects = audit_ledgers(self.before(), after)
        self.assertTrue(any("changed identity" in defect for defect in defects), defects)

    def test_a_resized_stack_fails(self):
        after = self.before()
        after["items"]["wizard_spell_book"]["quantity"] = 2
        defects = audit_ledgers(self.before(), after)
        self.assertTrue(any("changed quantity" in defect for defect in defects), defects)

    def test_minted_and_burned_gold_fail(self):
        minted = self.before()
        minted["gold"]["ground:forged"] = 1
        minted["gold_total"] += 1
        self.assertTrue(
            any("created or destroyed" in defect for defect in audit_ledgers(self.before(), minted))
        )
        burned = self.before()
        burned["gold"]["actor:proof/c:sack"] -= 1
        burned["gold_total"] -= 1
        self.assertTrue(
            any("created or destroyed" in defect for defect in audit_ledgers(self.before(), burned))
        )


class AnsweringTheBrowserHandshake(unittest.TestCase):
    """The runner's half, driven directly, with the database read replaced."""

    def audit(self, documents):
        reading = LedgerAudit("postgresql://scripted")
        pending = list(documents)

        def stored(_url):
            return pending.pop(0)

        import run_death_return_proof as proof

        original = proof.stored_checkpoint
        proof.stored_checkpoint = stored
        try:
            captured = reading.answer({"action": "capture", "label": "before-death"})
            audited = reading.answer({"action": "audit", "label": "before-death"})
        finally:
            proof.stored_checkpoint = original
        return captured, audited

    def test_a_capture_then_an_audit_of_the_same_world_is_clean(self):
        document = checkpoint(carried_items={"right_hand": "weathered_staff"}, carried_gold=100)
        captured, audited = self.audit([document, document])
        self.assertEqual(captured["verdict"], "captured")
        self.assertEqual(captured["item_count"], 1)
        self.assertEqual(captured["gold_total"], 100)
        self.assertEqual(audited["verdict"], "audited")
        self.assertEqual(audited["defects"], [])

    def test_an_audit_without_a_capture_is_refused_rather_than_compared_with_nothing(self):
        reading = LedgerAudit("postgresql://scripted")
        refused = reading.answer({"action": "audit", "label": "never-captured"})
        self.assertEqual(refused["verdict"], "refused")
        self.assertIn("no ledger was captured", refused["defects"][0])

    def test_an_unknown_action_is_refused(self):
        reading = LedgerAudit("postgresql://scripted")
        refused = reading.answer({"action": "delete", "label": "before-death"})
        self.assertEqual(refused["verdict"], "refused")
        self.assertIn("unknown ledger action", refused["defects"][0])

    def test_an_audit_of_an_altered_world_reports_the_change(self):
        before = checkpoint(carried_items={"right_hand": "weathered_staff"}, carried_gold=100)
        after = checkpoint(
            carried_items={"right_hand": "weathered_staff"},
            carried_gold=100,
            instances={"forged": {"definition_id": "item/forged", "quantity": 1}},
            corpses={"corpse:1": {"contents": {"sack_item_1": "forged"}, "gold": 0}},
        )
        _, audited = self.audit([before, after])
        self.assertEqual(audited["verdict"], "audited")
        self.assertTrue(
            any("appeared from nowhere" in defect for defect in audited["defects"]),
            audited["defects"],
        )
        self.assertEqual(audited["after"]["item_count"], 2)

    def test_the_handshake_answer_is_json(self):
        document = checkpoint()
        captured, audited = self.audit([document, document])
        json.dumps({"captured": captured, "audited": audited})


if __name__ == "__main__":
    unittest.main()
