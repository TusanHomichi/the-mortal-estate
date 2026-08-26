"""The lane split, asserted.

The charter's load-bearing rule is that a full workspace build must not be the
price of inspecting a small change. The first test in `TheFastLane` is that
rule, stated as an assertion: a documentation-only change resolves to a plan
with no cargo command anywhere in it.

The second half of the rule matters just as much — the fast lane must not
silently *skip* something either. So every escalation is explicit, reasoned,
and tested.
"""

from __future__ import annotations

import unittest

from verification_test_support import REPO_ROOT  # noqa: F401  (path setup)

from verification import resolve, table
from verification.model import ResolutionError


def commands(scopes) -> list[str]:
    return [" ".join(step.argv) for step in resolve.steps_for(scopes)]


class TheFastLane(unittest.TestCase):
    def test_a_documentation_change_costs_no_cargo_command(self) -> None:
        selection = resolve.select_changed_paths(["docs/boundary-map.md"])
        self.assertFalse(selection.escalated)
        plan = commands(selection.scopes)
        self.assertTrue(plan)
        for command in plan:
            self.assertNotIn("cargo", command, command)

    def test_a_documentation_change_still_runs_the_boundary_checks(self) -> None:
        selection = resolve.select_changed_paths(["AGENTS.md"])
        keys = {step.key for step in resolve.steps_for(selection.scopes)}
        self.assertIn("boundary.banned_terms", keys)
        self.assertIn("docs.markdown_links", keys)

    def test_a_documentation_change_costs_no_client_run(self) -> None:
        selection = resolve.select_changed_paths(["README.md"])
        keys = {step.key for step in resolve.steps_for(selection.scopes)}
        self.assertFalse({key for key in keys if key.startswith("client.")})

    def test_a_rust_change_does_select_the_workspace(self) -> None:
        selection = resolve.select_changed_paths(["crates/tme-rules/src/lib.rs"])
        keys = {step.key for step in resolve.steps_for(selection.scopes)}
        self.assertIn("rust.build", keys)
        self.assertFalse({key for key in keys if key.startswith("client.")})

    def test_a_client_change_selects_the_client_and_not_the_workspace(self) -> None:
        selection = resolve.select_changed_paths(["client/tests/run_all.gd"])
        keys = {step.key for step in resolve.steps_for(selection.scopes)}
        self.assertIn("client.suite", keys)
        self.assertNotIn("rust.build", keys)

    def test_a_python_change_selects_the_suite_and_the_checks(self) -> None:
        selection = resolve.select_changed_paths(["tools/check_hostnames.py"])
        keys = {step.key for step in resolve.steps_for(selection.scopes)}
        self.assertIn("python.boundary", keys)
        self.assertIn("boundary.hostnames", keys)
        self.assertNotIn("rust.build", keys)

    def test_the_meta_check_runs_in_every_fast_resolution(self) -> None:
        for path in ("docs/server-notes.md", "crates/tme-sim/src/loading.rs", "client/project.godot"):
            selection = resolve.select_changed_paths([path])
            self.assertIn("meta", selection.scopes, path)

    def test_the_owner_invoked_lane_is_never_selected_automatically(self) -> None:
        for path in ("docs/workbench-v0.md", "tools/workbench/capture.py", "client/project.godot"):
            selection = resolve.select_changed_paths([path])
            self.assertNotIn("capture", selection.scopes, path)


class Escalation(unittest.TestCase):
    def test_no_paths_escalates(self) -> None:
        selection = resolve.select_changed_paths([])
        self.assertTrue(selection.escalated)
        self.assertEqual(selection.scopes, ("portable",))

    def test_a_workflow_change_escalates(self) -> None:
        selection = resolve.select_changed_paths([".github/workflows/verify.yml"])
        self.assertTrue(selection.escalated)

    def test_changing_the_runner_itself_escalates(self) -> None:
        for path in ("tools/run_verification.py", "tools/verification/table.py", "Cargo.lock"):
            self.assertTrue(resolve.select_changed_paths([path]).escalated, path)

    def test_a_deleted_path_escalates(self) -> None:
        selection = resolve.select_changed_paths(["docs/this-was-deleted.md"])
        self.assertTrue(selection.escalated)
        self.assertIn("deleted or renamed", selection.reasons[0])

    def test_an_unrecognised_path_escalates(self) -> None:
        selection = resolve.select_changed_paths(["LICENSE.txt"])
        self.assertTrue(selection.escalated)

    def test_a_malformed_path_escalates(self) -> None:
        for bad in ("/etc/passwd", "../outside", "with\nnewline", ""):
            self.assertTrue(resolve.select_changed_paths([bad]).escalated, repr(bad))

    def test_an_escalation_always_says_why(self) -> None:
        selection = resolve.select_changed_paths([".cargo/config.toml"])
        self.assertTrue(selection.reasons and selection.reasons[0])


class TheExclusionRule(unittest.TestCase):
    def test_an_expensive_scope_without_a_cause_is_refused(self) -> None:
        """P9 for the lane split: the mutant is a fast plan that smuggles cargo in."""
        smuggled = resolve.Selection(("meta", "docs", "rust"), ("planted",))
        with self.assertRaises(ResolutionError) as raised:
            resolve.assert_fast_lane(smuggled, ["docs"])
        self.assertIn("with no changed path that needs it", str(raised.exception))

    def test_the_owner_invoked_lane_is_refused_in_a_fast_plan(self) -> None:
        smuggled = resolve.Selection(("meta", "capture"), ("planted",))
        with self.assertRaises(ResolutionError) as raised:
            resolve.assert_fast_lane(smuggled, ["docs"])
        self.assertIn("owner-invoked", str(raised.exception))

    def test_a_caused_scope_is_allowed(self) -> None:
        resolve.assert_fast_lane(resolve.Selection(("meta", "rust"), ()), ["rust"])

    def test_the_forbidden_set_names_everything_expensive(self) -> None:
        self.assertEqual(
            table.FAST_FORBIDDEN_WITHOUT_CAUSE,
            {"rust", "client", "gated", "cleanclone", "capture"},
        )


class Classification(unittest.TestCase):
    def test_families_are_what_they_look_like(self) -> None:
        cases = {
            "docs/boundary-map.md": "docs",
            "AGENTS.md": "docs",
            "crates/tme-server/src/main.rs": "rust",
            ".sqlx/query-abc.json": "rust",
            "client/presentation/grid_world_view.gd": "client",
            "tools/check_hostnames.py": "python",
            "tests/test_run_checks.py": "python",
            "tools/workbench/resolve.py": "workbench",
            "tools/hostname-allowlist.txt": "boundary-data",
            "content/test-corpus/catalogs/prototype_catalog_v6.json": "content",
        }
        for path, family in cases.items():
            self.assertEqual(resolve.classify(path), family, path)

    def test_an_unknown_path_classifies_as_nothing(self) -> None:
        self.assertIsNone(resolve.classify("something/unheard/of.bin"))


if __name__ == "__main__":
    unittest.main()
