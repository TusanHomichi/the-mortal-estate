"""The step table's shape, asserted so it cannot drift.

Three claims live here, and each one is a defect class the predecessor's runner
actually suffered:

* the owner scopes **partition** the table — no step belongs to two lanes, and
  none belongs to none;
* `full` covers everything except the declared owner-invoked lane, so the
  complete lane cannot quietly stop covering something;
* the Python test ownership inventory **fails closed** — an unclassified module
  makes every scope refuse to resolve, rather than being a test nobody runs.
"""

from __future__ import annotations

import tempfile
import unittest
from pathlib import Path

from verification_test_support import REPO_ROOT  # noqa: F401  (path setup)

import run_checks
from verification import resolve, table
from verification.model import ResolutionError


class ThePartition(unittest.TestCase):
    def test_every_step_is_owned_by_exactly_one_owner_scope(self) -> None:
        for key, step in table.STEPS.items():
            self.assertIn(step.owner, table.OWNER_SCOPES, key)

    def test_the_owner_scopes_cover_the_whole_table(self) -> None:
        owned = {step.key for step in table.STEPS.values()}
        covered: set[str] = set()
        for scope in table.OWNER_SCOPES:
            members = {step.key for step in table.STEPS.values() if step.owner == scope}
            self.assertFalse(covered & members, f"{scope} overlaps an earlier scope")
            covered |= members
        self.assertEqual(owned, covered)

    def test_full_is_every_step_except_the_owner_invoked_lane(self) -> None:
        full = {step.key for step in resolve.steps_for(["full"])}
        out_of_baseline = {
            step.key for step in table.STEPS.values() if step.owner in table.OUT_OF_BASELINE
        }
        self.assertEqual(full | out_of_baseline, set(table.STEPS))
        self.assertFalse(full & out_of_baseline)

    def test_full_contains_portable(self) -> None:
        portable = {step.key for step in resolve.steps_for(["portable"])}
        full = {step.key for step in resolve.steps_for(["full"])}
        self.assertTrue(portable < full)

    def test_the_web_scope_is_in_full_and_runs_install_first(self) -> None:
        web = [step.key for step in resolve.steps_for(["web"])]
        full = {step.key for step in resolve.steps_for(["full"])}
        self.assertEqual(web, ["web.install", "web.typecheck", "web.test", "web.build"])
        self.assertTrue(set(web) <= full)

    def test_the_capture_lane_is_the_only_thing_outside_the_baseline(self) -> None:
        self.assertEqual(table.OUT_OF_BASELINE, {"capture"})

    def test_the_cargo_owners_are_owner_scopes_that_actually_build(self) -> None:
        """`--report-disk` measures after these; a stale name measures nothing."""
        self.assertTrue(set(table.CARGO_OWNERS) <= set(table.OWNER_SCOPES))
        for scope in table.CARGO_OWNERS:
            commands = " ".join(
                " ".join(step.argv)
                for step in table.STEPS.values()
                if step.owner == scope
            )
            self.assertTrue(
                "cargo" in commands or "clean_clone" in commands or "rust_tests" in commands,
                f"{scope} is listed as building but runs nothing that does",
            )

    def test_no_composed_scope_names_a_scope_that_does_not_exist(self) -> None:
        for name, members in table.COMPOSED.items():
            for member in members:
                bare = member[1:] if member.startswith("@") else member
                self.assertIn(bare, (*table.OWNER_SCOPES, *table.COMPOSED), f"{name} -> {member}")


class TheBoundaryRegistry(unittest.TestCase):
    def test_every_registered_check_becomes_a_step(self) -> None:
        registered = {check.name for check in run_checks.CHECKS}
        stepped = {
            step.label.split(": ", 1)[1]
            for step in table.STEPS.values()
            if step.key.startswith(("boundary.", "docs.")) and ": " in step.label
        }
        self.assertTrue(registered <= stepped, f"missing steps for {registered - stepped}")

    def test_the_link_check_is_owned_by_the_docs_lane(self) -> None:
        step = table.STEPS["docs.markdown_links"]
        self.assertEqual(step.owner, "docs")

    def test_only_the_terms_check_degrades(self) -> None:
        degrading = {step.key for step in table.STEPS.values() if step.degrades_without}
        self.assertEqual(degrading, {"boundary.banned_terms"})
        self.assertEqual(table.STEPS["boundary.banned_terms"].degrades_without, "private-terms")


class PythonOwnership(unittest.TestCase):
    def test_the_inventory_matches_this_tree(self) -> None:
        resolve.validate_python_ownership()

    def test_an_unclassified_module_fails_closed(self) -> None:
        """P9: the mutant is a test module nobody classified."""
        with tempfile.TemporaryDirectory() as scratch:
            tests = Path(scratch) / "tests"
            tests.mkdir()
            for module in (
                name
                for modules in table.PYTHON_TEST_OWNERS.values()
                for name in modules
            ):
                (tests / (module.split(".")[-1] + ".py")).write_text("", encoding="utf-8")
            (tests / "test_nobody_classified_me.py").write_text("", encoding="utf-8")
            with self.assertRaises(ResolutionError) as raised:
                resolve.validate_python_ownership(root=Path(scratch))
        self.assertIn("unclassified=['tests.test_nobody_classified_me']", str(raised.exception))

    def test_a_classified_module_that_vanished_fails_closed(self) -> None:
        with tempfile.TemporaryDirectory() as scratch:
            (Path(scratch) / "tests").mkdir()
            with self.assertRaises(ResolutionError) as raised:
                resolve.validate_python_ownership(root=Path(scratch))
        self.assertIn("classified-but-missing=", str(raised.exception))

    def test_a_module_classified_twice_fails_closed(self) -> None:
        original = dict(table.PYTHON_TEST_OWNERS)
        try:
            table.PYTHON_TEST_OWNERS["boundary"] = (
                *original["boundary"],
                original["workbench"][0],
            )
            with self.assertRaises(ResolutionError) as raised:
                resolve.validate_python_ownership()
        finally:
            table.PYTHON_TEST_OWNERS.clear()
            table.PYTHON_TEST_OWNERS.update(original)
        self.assertIn("classified twice", str(raised.exception))

    def test_every_classified_module_exists(self) -> None:
        for modules in table.PYTHON_TEST_OWNERS.values():
            for module in modules:
                path = REPO_ROOT / (module.replace(".", "/") + ".py")
                self.assertTrue(path.is_file(), module)


class ScopeExpansion(unittest.TestCase):
    def test_expansion_is_ordered_and_deduplicated(self) -> None:
        self.assertEqual(
            resolve.expand(["portable", "python"])[:2], ("meta", "docs")
        )
        self.assertEqual(len(set(resolve.expand(["full"]))), len(resolve.expand(["full"])))

    def test_an_unknown_scope_is_a_resolution_error(self) -> None:
        with self.assertRaises(ResolutionError):
            resolve.expand(["not-a-scope"])


if __name__ == "__main__":
    unittest.main()
