"""The live proof rejects shared-phase and truncated action timing."""
import contextlib
import io
import unittest
import boundary_test_support  # noqa: F401
import run_server_live_proof as live_proof
from live_server_harness import ProofError


def observations(*rows):
    return '\n'.join(f'cooldown_observation = start {start} ready {ready} elapsed {elapsed} ms' for start, ready, elapsed in rows)


class CooldownVerdictTests(unittest.TestCase):
    def test_independent_offsets_and_full_durations_pass(self):
        with contextlib.redirect_stdout(io.StringIO()):
            live_proof.check_cooldowns(observations((4127, 7127, 3020), (8300, 11300, 3030), (13811, 16811, 3010)))

    def test_missing_shortened_shared_phase_and_warped_intervals_are_refused(self):
        for rows in [[], [(4127, 7127, 3020)], [(4127, 6000, 1900)] * 3, [(3000, 6000, 3000), (6000, 9000, 3000), (9000, 12000, 3000)], [(4127, 7127, 20)] * 3]:
            with self.assertRaises(ProofError):
                live_proof.check_cooldowns(observations(*rows))
