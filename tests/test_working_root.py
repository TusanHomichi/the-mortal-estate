"""Owner ruling D6, asserted: the ignored working root is never load-bearing.

`docs/working-root-policy.md` states the six requirements. Three of them are
statements about *this tree* rather than about a document, and those three are
proven here — the rest point at proofs that already exist elsewhere and are
named in the policy's own table.

The claim that matters most is the first one: **no tracked proof reads the
ignored root.** It is asserted against the resolved step table rather than by
reading the tools, because the step table is what a verification run actually
executes.
"""

from __future__ import annotations

import subprocess
import unittest
from pathlib import Path

from verification_test_support import REPO_ROOT

import run_clean_clone_proof

from verification import resolve
from verification.table import STEPS

#: Every root `.gitignore` declares, read from the file. Not restated: a second
#: list of private roots in a carried file is exactly what the clean-room check
#: exists to prevent, and it would drift the first time a root was added.
IGNORED_ROOTS = run_clean_clone_proof.forbidden_roots(REPO_ROOT)

#: The tracked fixtures a clean clone carries instead. Named here so the policy
#: document's claim "they do — name them" stays checkable.
TRACKED_FIXTURES = (
    "tests/fixtures/synthetic-terms.txt",
    "tests/fixtures/workbench/regenerate.py",
    "tests/fixtures/workbench/session/manifest.json",
    "tests/fixtures/capture/fixture-route/capture.png",
    "tests/fixtures/capture/live-route/capture.png",
    "tests/fixtures/capture/provenance.md",
    "tools/ci-synthetic-banned-terms.txt",
)


class TrackedProofNeverDependsOnTheIgnoredRoot(unittest.TestCase):
    def test_no_step_in_the_complete_lane_names_an_ignored_root(self) -> None:
        for step in resolve.steps_for(["full"]):
            for token in (*step.argv, *(step.degraded_argv or ())):
                for root in IGNORED_ROOTS:
                    self.assertNotIn(
                        root + "/", token, f"{step.key} names {root}/ in {token!r}"
                    )

    def test_no_step_at_all_names_an_ignored_root(self) -> None:
        """Including the owner-invoked capture lane: it writes there, never reads."""
        for step in STEPS.values():
            for token in step.argv:
                for root in IGNORED_ROOTS:
                    self.assertNotIn(root + "/", token, step.key)

    def test_the_complete_lane_resolves_with_every_ignored_root_absent(self) -> None:
        """A clean clone has none of them; resolution must not care."""
        steps = resolve.steps_for(["full"])
        self.assertTrue(steps)
        for step in steps:
            for token in step.argv:
                if token.startswith("$") or "/" not in token:
                    continue
                self.assertFalse(
                    any(token.startswith(root + "/") for root in IGNORED_ROOTS), step.key
                )


class CleanClonesCarryTrackedFixtures(unittest.TestCase):
    def test_every_named_fixture_is_carried_by_git(self) -> None:
        tracked = set(
            subprocess.run(
                ["git", "-C", str(REPO_ROOT), "ls-files", "-z"],
                capture_output=True,
                text=True,
                check=True,
            ).stdout.split("\0")
        )
        for fixture in TRACKED_FIXTURES:
            self.assertIn(fixture, tracked, fixture)

    def test_the_named_fixtures_exist_on_disk(self) -> None:
        for fixture in TRACKED_FIXTURES:
            self.assertTrue((REPO_ROOT / fixture).is_file(), fixture)


class TheIgnoredRootsAreActuallyIgnored(unittest.TestCase):
    def test_git_refuses_to_add_anything_under_them(self) -> None:
        for root in IGNORED_ROOTS:
            completed = subprocess.run(
                ["git", "-C", str(REPO_ROOT), "check-ignore", "-q", "--", f"{root}/probe"],
                capture_output=True,
                text=True,
                check=False,
            )
            self.assertEqual(completed.returncode, 0, f"{root}/ is not ignored")


class ThePolicyDocumentExists(unittest.TestCase):
    def test_the_policy_names_every_requirement(self) -> None:
        policy = (REPO_ROOT / "docs/working-root-policy.md").read_text(encoding="utf-8")
        for phrase in (
            "never depend on the ignored root",
            "tracked synthetic fixtures",
            "honest unavailable",
            "source hashes",
            "Retention",
            "promotion path",
        ):
            self.assertIn(phrase, policy, phrase)

    def test_the_policy_is_carried(self) -> None:
        self.assertTrue((REPO_ROOT / "docs/working-root-policy.md").is_file())


class NoTrackedLoaderReadsTheIgnoredRoot(unittest.TestCase):
    """The fail-closed half of D6 requirement four.

    A session file carrying digests is worthless if some tracked loader can be
    pointed at the session directory and read one as runtime input. So the set
    of carried files allowed to name the session root at all is small and
    fixed: the package that owns sessions, its scripted proof — which seeds a
    fresh session from the *tracked* capture fixture and therefore works on a
    clean clone — the tests, and the documents that state the policy.
    """

    ALLOWED = (
        "tools/workbench/",
        "tools/workbench_demo.py",
        "tests/",
        "docs/",
        ".gitignore",
        "AGENTS.md",
    )

    def test_only_the_workbench_and_its_documentation_name_the_session_root(self) -> None:
        completed = subprocess.run(
            ["git", "-C", str(REPO_ROOT), "grep", "-lF", "--", ".workbench/"],
            capture_output=True,
            text=True,
            check=False,
        )
        offenders = [
            name
            for name in completed.stdout.split()
            if name and not name.startswith(self.ALLOWED)
        ]
        self.assertEqual(offenders, [], f"these carried files name the session root: {offenders}")

    def test_neither_the_workspace_nor_the_client_names_the_session_root(self) -> None:
        completed = subprocess.run(
            ["git", "-C", str(REPO_ROOT), "grep", "-lF", "--", ".workbench/", "--", "crates/", "client/"],
            capture_output=True,
            text=True,
            check=False,
        )
        self.assertEqual(completed.stdout.split(), [])


if __name__ == "__main__":
    unittest.main()
