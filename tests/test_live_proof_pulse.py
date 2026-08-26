"""Tests for the live proof's judgement of the authoritative pulse.

The live proof itself needs PostgreSQL, TLS, credentials, and the pinned Godot
binary, so it runs on demand rather than in the standing suite. Its *verdict*
needs none of that, and the verdict is the part that must not quietly go soft:
a judge that accepts anything would keep printing green through the exact
regression ruling D5 exists to prevent. These cases feed it recorded
observations and check what it refuses.
"""

from __future__ import annotations

import contextlib
import io
import unittest

import boundary_test_support  # noqa: F401  (puts tools/ on sys.path)
import run_client_live_proof as live_proof
from live_server_harness import ProofError


def observations(*beats: tuple[int, int]) -> str:
    """Client output holding one `pulse_observation` line per beat."""
    return "\n".join(
        f"pulse_observation = T{logical} at {wall} ms" for logical, wall in beats
    )


def judge(stdout: str) -> str:
    captured = io.StringIO()
    with contextlib.redirect_stdout(captured):
        live_proof.check_pulse(stdout)
    return captured.getvalue()


class PulseVerdictTests(unittest.TestCase):
    def test_the_ruled_cadence_passes_with_ordinary_host_jitter(self) -> None:
        report = judge(observations((3, 5898), (4, 8916), (5, 11922), (6, 14915)))
        self.assertIn("T3 -> T4 in 3018 ms", report)
        self.assertIn("-7 ms", report)  # a short interval reports as short
        self.assertIn("4 beats observed", report)

    def test_the_one_second_cadence_is_refused(self) -> None:
        with self.assertRaises(ProofError) as refusal:
            judge(observations((3, 1000), (4, 2001), (5, 3002)))
        self.assertIn("outside 3000 +/- 750 ms", str(refusal.exception))

    def test_a_beat_that_skips_a_round_is_refused(self) -> None:
        with self.assertRaises(ProofError) as refusal:
            judge(observations((3, 1000), (5, 4000)))
        self.assertIn("must advance exactly one round", str(refusal.exception))

    def test_a_stalled_pulse_is_refused_rather_than_read_as_success(self) -> None:
        for stdout in ("", "ok: nothing about the pulse", observations((3, 1000))):
            with self.subTest(stdout=stdout):
                with self.assertRaises(ProofError) as refusal:
                    judge(stdout)
                self.assertIn("at least two are needed", str(refusal.exception))

    def test_the_expected_pulse_is_the_ruled_three_seconds(self) -> None:
        self.assertEqual(3000.0, live_proof.RULED_PULSE_MSEC)
        self.assertLess(
            live_proof.PULSE_TOLERANCE_MSEC,
            live_proof.RULED_PULSE_MSEC - 1000.0,
            "the tolerance must stay narrow enough to exclude a one-second cadence",
        )


if __name__ == "__main__":
    unittest.main()
