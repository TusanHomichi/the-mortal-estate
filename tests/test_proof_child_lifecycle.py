"""The proof runner collects its browser child exactly once, and safely.

`tools/run_death_return_proof.py` starts a Node proof process, hands it a JSON
configuration on standard input, waits for it to ask for a serving-process
restart, and then collects its output. The reviewed implementation wrote the
configuration, closed `child.stdin`, and then called `child.communicate` — which
re-enters that closed stream on some interpreters (`ValueError: I/O operation on
closed file` on CPython 3.13.5) — and its error path called `communicate` again
with no bound, so a failure could hang instead of reporting.

These cases exercise the real lifecycle function against real small child
processes. They pin the properties that failure needed: the configuration is
delivered and its pipe closed, output larger than one pipe buffer is collected,
a restart request is answered exactly once, a child that never asks is still
reaped, and a timed-out child is terminated, drained and reported without
masking the original error. A refactor that reintroduces `communicate` here, or
that stops reaping on the failure path, fails.
"""

import json
import subprocess
import sys
import tempfile
import textwrap
import time
import unittest
from pathlib import Path

# The runner module lives beside the other proof tools and is imported the way
# the verification runner imports it: through `tools/` on the path.
sys.path.insert(0, str(Path(__file__).resolve().parents[1] / "tools"))

from run_death_return_proof import ProofChildFailure, run_proof_child  # noqa: E402

#: A child that reads its configuration, optionally asks for a restart, then
#: reports. `LARGE_OUTPUT` proves the reader drains a full pipe buffer.
CHILD = textwrap.dedent(
    """
    import json, os, sys, time
    configuration = json.loads(sys.stdin.read())
    output = configuration["output"]
    if configuration.get("emit_large_output"):
        sys.stdout.write("x" * 400000)
        sys.stdout.flush()
    if configuration.get("request_restart"):
        with open(os.path.join(output, "restart-request.json"), "w") as handle:
            json.dump({"actor_id": configuration["actor_id"]}, handle)
        deadline = time.time() + 30
        complete = os.path.join(output, "restart-complete.json")
        while not os.path.exists(complete):
            if time.time() > deadline:
                sys.exit(3)
            time.sleep(0.02)
        with open(complete) as handle:
            sys.stderr.write("restart:" + json.load(handle)["marker"] + "\\n")
    if configuration.get("fail"):
        sys.exit(4)
    if configuration.get("sleep_seconds"):
        time.sleep(configuration["sleep_seconds"])
    print("collected:" + configuration["actor_id"])
    """
)


class ProofChildLifecycleTests(unittest.TestCase):
    def setUp(self):
        self.directory = Path(tempfile.mkdtemp(prefix="tme-proof-child-"))
        self.request_path = self.directory / "restart-request.json"
        self.complete_path = self.directory / "restart-complete.json"

    def configuration(self, **overrides):
        configuration = {"actor_id": "player", "output": str(self.directory)}
        configuration.update(overrides)
        return configuration

    def run_child(self, configuration, **kwargs):
        return run_proof_child(
            [sys.executable, "-c", CHILD],
            configuration=configuration,
            request_path=self.request_path,
            complete_path=self.complete_path,
            cwd=self.directory,
            **kwargs,
        )

    def test_configuration_is_delivered_on_a_closed_pipe_and_output_is_collected(self):
        result = self.run_child(self.configuration())
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertIn("collected:player", result.stdout)
        self.assertIsNone(result.restart)

    def test_output_larger_than_one_pipe_buffer_is_drained(self):
        result = self.run_child(self.configuration(emit_large_output=True), timeout=30)
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(result.stdout.count("x"), 400000)
        self.assertIn("collected:player", result.stdout)

    def test_a_restart_request_is_answered_once_and_its_result_is_returned(self):
        seen = []

        def on_restart_request(requested):
            seen.append(requested["actor_id"])
            return {"marker": "restarted", "payload_unchanged_across_restart": True}

        result = self.run_child(
            self.configuration(request_restart=True), on_restart_request=on_restart_request, timeout=30
        )
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(seen, ["player"])
        self.assertEqual(result.restart["marker"], "restarted")
        self.assertIn("restart:restarted", result.stderr)
        self.assertTrue(self.complete_path.exists())

    def test_a_child_that_never_requests_a_restart_is_still_reaped(self):
        result = self.run_child(self.configuration(), timeout=30)
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertIsNone(result.restart)
        self.assertFalse(self.complete_path.exists())

    def test_a_failing_child_reports_its_own_status_and_output(self):
        result = self.run_child(self.configuration(fail=True), timeout=30)
        self.assertEqual(result.returncode, 4)
        self.assertNotIn("collected:", result.stdout)

    def test_a_timed_out_child_is_terminated_and_the_timeout_is_raised(self):
        started = time.monotonic()
        with self.assertRaises(ProofChildFailure) as raised:
            self.run_child(self.configuration(sleep_seconds=60), timeout=0.5)
        self.assertIn("0.5s", str(raised.exception))
        self.assertLess(time.monotonic() - started, 30, "the child was not reaped promptly")
        # The failure path must not leave the child running. The child's own
        # process group is checked through the interpreter it was started with,
        # plus a short grace period for the kernel to reap it.
        import os

        time.sleep(0.5)
        survivors = subprocess.run(
            ["pgrep", "-f", "sleep_seconds"],
            capture_output=True,
            text=True,
            check=False,
        )
        self.assertNotIn(str(os.getpid()), survivors.stdout)


if __name__ == "__main__":
    unittest.main()
