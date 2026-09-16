"""Restart receipts are derived from observations, not asserted into existence.

`tools/run_death_return_proof.py` restarts the serving process under a dead
character and reports what the restart did to the stored checkpoint and to the
authoritative answers a fresh session receives. The reviewed implementation
required `show_sack` to be *rejected* and then returned
`"accepted_intent": {"kind": "show_sack"}, "accepted_command": True` — a
successful path that emitted a false acceptance claim — and hardcoded
`payload_unchanged_across_restart: True` for a comparison it never ran as
stated.

These cases drive the real judging functions and the real
`prove_restart_durability` with scripted observations, so a receipt field cannot
claim a result its inputs do not support:

* byte/digest preservation and semantic preservation are separate claims, and
  each is reported as what it is;
* a permitted presence movement (connected, absent_since) is accepted while an
  unexplained one (a character appearing, vanishing, or a control epoch moving)
  is refused;
* a dead actor's physical action that was *accepted* cannot be reported as
  refused, and an early return or a wrongly refused one is a named defect;
* an observation that contradicts the claim raises instead of producing a
  success-looking receipt.
"""

import sys
import unittest
from pathlib import Path
from unittest import mock

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / "tools"))

import run_death_return_proof as proof  # noqa: E402

PLAYER = "player"
CHARACTER = "proof/character"

#: A dead character's durable facts, in the shape `observe_frame` returns.
DEATH_FRAME = {
    "observer_actor_id": PLAYER,
    "actors": [{"actor_id": PLAYER, "life_state": "ghost", "hp": 0}],
    "observation_center": {"realm": "first_expedition", "level": "temple", "x": 3, "y": 6},
    "social": {"character_id": CHARACTER},
    "carried": {"gold": {"sack": 12}, "items": []},
    "character": {"skill_ledger": []},
    "ready_at": "100",
    "logical_time": "40",
    "can_act": False,
}


def death_frame(**overrides):
    frame = {**DEATH_FRAME, "actors": [dict(DEATH_FRAME["actors"][0])]}
    frame.update(overrides)
    return frame


def observation(logical_time="200", **overrides):
    """One observation of the same ghost, by default past its own deadline."""
    frame = death_frame(logical_time=logical_time, **overrides)
    return proof.observe_frame(frame)


def attempts(return_kind="accepted", physical_kind="rejected"):
    return {
        "request_resurrection": {
            "disposition": {"kind": return_kind},
            "command_id": "command/1",
            "server_sequence": 42,
        },
        "show_sack": {
            "disposition": {"kind": physical_kind},
            "command_id": "command/2",
            "server_sequence": 43,
        },
    }


class ComparingCheckpoints(unittest.TestCase):
    def compare(self, gameplay_before, gameplay_after, volatile_before=None, volatile_after=None):
        return proof.compare_checkpoints(
            digest_before="digest-before",
            digest_after="digest-after",
            gameplay_before=gameplay_before,
            gameplay_after=gameplay_after,
            volatile_before=volatile_before or {},
            volatile_after=volatile_after or {},
        )

    def test_identical_gameplay_payloads_are_reported_as_semantically_unchanged(self):
        receipt = self.compare('{"world": {"actors": []}}', '{"world": {"actors": []}}')
        self.assertTrue(receipt["durable_payload_unchanged"])
        self.assertEqual(receipt["defects"], [])
        # The raw digest moved. That is reported, not hidden and not asserted away.
        self.assertFalse(receipt["checkpoint_bytes_identical"])
        self.assertEqual(
            receipt["payload_comparison"], "parsed_json_minus_named_volatile_fields"
        )

    def test_an_altered_gameplay_payload_is_a_named_defect(self):
        receipt = self.compare('{"world": {"location": "a"}}', '{"world": {"location": "b"}}')
        self.assertFalse(receipt["durable_payload_unchanged"])
        self.assertEqual(len(receipt["defects"]), 1)
        self.assertIn("durable checkpoint payload changed", receipt["defects"][0])

    def test_a_permitted_presence_movement_is_not_a_defect(self):
        receipt = self.compare(
            "{}",
            "{}",
            volatile_before={"character_presence": {
                CHARACTER: {"connected": True, "absent_since": None, "control_epoch": 4}}},
            volatile_after={"character_presence": {
                CHARACTER: {"connected": False, "absent_since": "41", "control_epoch": 4}}},
        )
        self.assertEqual(receipt["defects"], [])
        self.assertTrue(receipt["volatile_presence_movement"])

    def test_a_moved_control_epoch_is_an_unexplained_presence_change(self):
        receipt = self.compare(
            "{}",
            "{}",
            volatile_before={"character_presence": {
                CHARACTER: {"connected": True, "absent_since": None, "control_epoch": 4}}},
            volatile_after={"character_presence": {
                CHARACTER: {"connected": False, "absent_since": None, "control_epoch": 5}}},
        )
        self.assertEqual(len(receipt["defects"]), 1)
        self.assertIn("control epoch", receipt["defects"][0])

    def test_a_character_appearing_in_the_presence_ledger_is_refused(self):
        receipt = self.compare(
            "{}",
            "{}",
            volatile_before={"character_presence": {}},
            volatile_after={"character_presence": {
                CHARACTER: {"connected": True, "absent_since": None, "control_epoch": 1}}},
        )
        self.assertIn("presence mark", receipt["defects"][0])


class JudgingDeadActorAttempts(unittest.TestCase):
    def test_a_refused_action_and_an_eligible_return_are_reported_as_observed(self):
        verdict = proof.judge_dead_actor_attempts(
            attempts(), logical_time="200", ready_at="100"
        )
        self.assertEqual(verdict["defects"], [])
        self.assertTrue(verdict["physical_action_refused_while_dead"])
        self.assertTrue(verdict["return_accepted"])
        self.assertFalse(verdict["return_refused_before_deadline"])
        self.assertEqual(
            verdict["eligibility_evaluated_at"], {"logical_time": "200", "ready_at": "100"}
        )

    def test_an_accepted_action_for_a_dead_actor_cannot_be_reported_as_refused(self):
        verdict = proof.judge_dead_actor_attempts(
            attempts(physical_kind="accepted"), logical_time="200", ready_at="100"
        )
        self.assertFalse(verdict["physical_action_refused_while_dead"])
        self.assertIn("not refused", verdict["defects"][0])

    def test_an_accepted_return_before_the_deadline_is_a_defect(self):
        verdict = proof.judge_dead_actor_attempts(
            attempts(return_kind="accepted"), logical_time="99", ready_at="100"
        )
        self.assertFalse(verdict["return_refused_before_deadline"])
        self.assertIn("before the character's own deadline", verdict["defects"][0])

    def test_a_refused_return_after_the_deadline_is_a_defect(self):
        verdict = proof.judge_dead_actor_attempts(
            attempts(return_kind="rejected"), logical_time="100", ready_at="100"
        )
        self.assertIn("refused a return", verdict["defects"][0])

    def test_a_refused_return_before_the_deadline_is_the_correct_answer(self):
        verdict = proof.judge_dead_actor_attempts(
            attempts(return_kind="rejected"), logical_time="99", ready_at="100"
        )
        self.assertEqual(verdict["defects"], [])
        self.assertFalse(verdict["return_accepted"])
        self.assertTrue(verdict["return_refused_before_deadline"])

    def test_an_unanswered_return_is_a_defect_rather_than_an_acceptance(self):
        unanswered = attempts()
        unanswered["request_resurrection"]["disposition"] = {}
        verdict = proof.judge_dead_actor_attempts(unanswered, logical_time="200", ready_at="100")
        self.assertFalse(verdict["return_accepted"])
        self.assertIn("did not answer a return request", verdict["defects"][0])


class ScriptedClient:
    """One authenticated session's frame and the answers it receives."""

    def __init__(self, frame, answers):
        self.frame = frame
        self.answers = answers
        self.intents = []
        self.enters = 0

    def __enter__(self):
        self.enters += 1
        return self

    def __exit__(self, *exit):
        return False

    def command(self, intent):
        self.intents.append(intent)
        return self.answers[intent["kind"]], []


class ProvingRestartDurability(unittest.TestCase):
    """The whole receipt, driven through the real entry point.

    `LiveWireClient` and the checkpoint read are replaced with scripted
    observations, so what is under test is exactly the derivation: the
    composition of the comparison and the judgment into the receipt the browser
    half asserts against.
    """

    def run_proof(self, *, after_frame, answers, gameplay_after=None,
                  volatile_before=None, volatile_after=None):
        client = ScriptedClient(after_frame, answers)
        self.client = client
        checkpoints = [
            ("digest-before", '{"world": {}}', volatile_before or {}),
            ("digest-after", gameplay_after or '{"world": {}}', volatile_after or {}),
        ]
        server = mock.Mock()
        server.database_url = "postgresql://scripted"
        with mock.patch.object(proof, "LiveWireClient", return_value=client), mock.patch.object(
            proof, "durable_checkpoint", side_effect=checkpoints
        ):
            receipt = proof.prove_restart_durability(
                server,
                expect={"actor_id": PLAYER, "character_id": CHARACTER, "location":
                        DEATH_FRAME["observation_center"], "ready_at": "100"},
            )
        self.assertEqual(
            [intent["kind"] for intent in client.intents],
            ["request_resurrection", "show_sack"],
            "the restarted session is asked exactly the two intents the receipt names",
        )
        return receipt

    def test_a_clean_restart_receipt_claims_only_what_was_observed(self):
        receipt = self.run_proof(
            after_frame=death_frame(logical_time="200"), answers=attempts()
        )
        self.assertTrue(receipt["durable_payload_unchanged"])
        self.assertTrue(receipt["physical_action_refused_while_dead"])
        self.assertTrue(receipt["return_accepted"])
        self.assertEqual(
            receipt["command_attempts"]["show_sack"]["disposition"]["kind"], "rejected"
        )
        self.assertEqual(receipt["command_attempts"]["show_sack"]["command_id"], "command/2")
        # No field may report a rejection as an accepted command.
        self.assertNotIn("accepted_command", receipt)
        self.assertNotIn("accepted_intent", receipt)
        self.assertFalse(receipt["return_refused_before_deadline"])

    def test_a_forged_acceptance_raises_instead_of_reaching_the_receipt(self):
        with self.assertRaises(RuntimeError) as raised:
            self.run_proof(
                after_frame=death_frame(logical_time="200"),
                answers=attempts(physical_kind="accepted"),
            )
        self.assertIn("not refused a physical or sheet action", str(raised.exception))

    def test_an_early_return_raises_instead_of_reaching_the_receipt(self):
        with self.assertRaises(RuntimeError) as raised:
            self.run_proof(
                after_frame=death_frame(logical_time="99"),
                answers=attempts(return_kind="accepted"),
            )
        self.assertIn("before the character's own deadline", str(raised.exception))

    def test_an_altered_gameplay_payload_raises_before_the_session_is_opened(self):
        with self.assertRaises(RuntimeError) as raised:
            self.run_proof(
                after_frame=death_frame(logical_time="200"),
                answers=attempts(),
                gameplay_after='{"world": {"corpses": {}}}',
            )
        self.assertIn("durable checkpoint payload changed", str(raised.exception))
        self.assertEqual(
            self.client.enters,
            1,
            "only the pre-restart observation may happen; the changed world must not be "
            "re-entered as an acceptable one",
        )
        self.assertEqual(self.client.intents, [], "no intent may be sent to the changed world")

    def test_an_unexplained_presence_change_raises(self):
        with self.assertRaises(RuntimeError) as raised:
            self.run_proof(
                after_frame=death_frame(logical_time="200"),
                answers=attempts(),
                volatile_before={"character_presence": {
                    CHARACTER: {"connected": True, "absent_since": None, "control_epoch": 4}}},
                volatile_after={"character_presence": {
                    CHARACTER: {"connected": False, "absent_since": "41", "control_epoch": 9}}},
            )
        self.assertIn("control epoch", str(raised.exception))


if __name__ == "__main__":
    unittest.main()
