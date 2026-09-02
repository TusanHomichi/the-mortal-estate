"""The step-table existence check, and its P9 mutant.

The predecessor's runner named a Python module that existed nowhere in its
repository, and one scope failed on every run because of it. The mutant here is
that exact defect, planted: a step naming a module that does not exist.
"""

from __future__ import annotations

import json
import unittest

from verification_test_support import REPO_ROOT  # noqa: F401  (path setup)

from verification import targets
from verification.model import Step
from verification.table import STEPS


class ThisTree(unittest.TestCase):
    def test_every_target_the_table_names_exists(self) -> None:
        self.assertEqual(targets.missing_targets(STEPS.values()), [])

    def test_the_check_reports_ok(self) -> None:
        self.assertEqual(targets.report(STEPS), 0)


class Mutants(unittest.TestCase):
    def test_mutant_step_naming_a_missing_module_is_killed(self) -> None:
        planted = Step(
            key="python.planted",
            owner="python",
            label="python: planted",
            argv=("python3", "-m", "unittest", "-q", "tests.test_this_does_not_exist"),
            mode="quiet_unittest",
        )
        problems = targets.missing_targets([planted])
        self.assertEqual(len(problems), 1, problems)
        self.assertIn("tests/test_this_does_not_exist.py", problems[0])

    def test_mutant_step_naming_a_missing_script_is_killed(self) -> None:
        planted = Step("x", "python", "x", ("python3", "tools/no_such_tool.py"))
        self.assertIn("tools/no_such_tool.py", targets.missing_targets([planted])[0])

    def test_mutant_step_naming_a_missing_client_script_is_killed(self) -> None:
        planted = Step("x", "client", "x", ("$TME_GODOT", "-s", "res://tests/no_such.gd"))
        self.assertIn("client/tests/no_such.gd", targets.missing_targets([planted])[0])

    def test_mutant_step_with_an_unknown_capability_is_killed(self) -> None:
        planted = Step("x", "client", "x", ("git", "diff"), requires=("teleportation",))
        self.assertIn("unknown capability", targets.missing_targets([planted])[0])

    def test_mutant_step_owned_by_a_scope_that_does_not_exist_is_killed(self) -> None:
        planted = Step("x", "nowhere", "x", ("git", "diff"))
        self.assertIn("is not an owner scope", targets.missing_targets([planted])[0])

    def test_mutant_in_a_degraded_argv_is_killed_too(self) -> None:
        planted = Step(
            "x",
            "boundary",
            "x",
            ("python3", "tools/check_hostnames.py"),
            degrades_without="private-terms",
            degraded_argv=("python3", "tools/check_hostnames.py", "--terms", "tools/gone.txt"),
            degraded_note="planted",
        )
        self.assertIn("tools/gone.txt", targets.missing_targets([planted])[0])

    def test_the_report_returns_non_zero_when_something_is_missing(self) -> None:
        planted = Step("x", "python", "x", ("python3", "tools/no_such_tool.py"))
        self.assertEqual(targets.report([planted]), 1)


class TokenReading(unittest.TestCase):
    def test_environment_references_are_not_treated_as_paths(self) -> None:
        step = Step("x", "client", "x", ("$TME_GODOT", "--headless"))
        self.assertEqual(targets.step_targets(step), [])

    def test_option_words_are_not_treated_as_paths(self) -> None:
        step = Step("x", "rust", "x", ("cargo", "build", "--workspace", "--locked"))
        self.assertEqual(targets.step_targets(step), [])

    def test_the_path_option_names_a_directory(self) -> None:
        step = Step("x", "client", "x", ("$TME_GODOT", "--path", "client", "--import"))
        self.assertEqual(targets.step_targets(step), ["client"])

    def test_the_npm_prefix_names_the_web_directory(self) -> None:
        step = Step("web.x", "web", "x", ("npm", "--prefix", "web", "run", "build"))
        self.assertEqual(targets.step_targets(step), ["web"])


class TheRustSchedulerGoesThroughCargo(unittest.TestCase):
    """The regression guard for the defect that cost this slice a red run.

    Executing the compiled test binaries directly bypasses cargo's `[env]`
    table, so `.cargo/config.toml` never applies and the tests run against a
    different denylist than they were written for. The Rust side has its own
    tripwire (`cargos_env_table_reaches_this_test_process`); this is the cheap
    one that fails in milliseconds.
    """

    def test_every_target_is_launched_by_cargo(self) -> None:
        from verification import rust_tests

        for kind, name in (("lib", "tme_rules"), ("test", "trace_json"), ("bin", "tme-sim")):
            command = rust_tests.TestTarget("tme-sim", kind, name).command("cargo")
            self.assertEqual(command[0], "cargo")
            self.assertEqual(command[1], "test")
            self.assertIn("--locked", command)
            self.assertNotIn("target/debug/deps", " ".join(command))

    def test_each_kind_gets_the_selector_cargo_expects(self) -> None:
        from verification import rust_tests

        self.assertEqual(rust_tests.TestTarget("p", "lib", "n").selector(), ("--lib",))
        self.assertEqual(rust_tests.TestTarget("p", "test", "n").selector(), ("--test", "n"))
        self.assertEqual(rust_tests.TestTarget("p", "bin", "n").selector(), ("--bin", "n"))

    def test_the_package_name_comes_from_the_manifest_path(self) -> None:
        from verification import rust_tests

        artifact = {
            "reason": "compiler-artifact",
            "profile": {"test": True},
            "executable": "/tmp/deps/tme_sim-abc",
            "manifest_path": "/repo/crates/tme-sim/Cargo.toml",
            "target": {"kind": ["lib"], "name": "tme_sim"},
        }
        (parsed,) = rust_tests.parse_artifacts(json.dumps(artifact))
        self.assertEqual(parsed.package, "tme-sim")
        self.assertEqual(parsed.label, "tme-sim::tme_sim")

    def test_an_artifact_stream_with_no_test_target_fails_closed(self) -> None:
        from verification import rust_tests

        with self.assertRaises(rust_tests.RustTestError):
            rust_tests.parse_artifacts('{"reason":"build-finished","success":true}')

    def test_a_non_json_artifact_line_fails_closed(self) -> None:
        from verification import rust_tests

        with self.assertRaises(rust_tests.RustTestError):
            rust_tests.parse_artifacts("not json at all")


class TheTripwireExists(unittest.TestCase):
    def test_the_rust_side_asserts_cargo_launched_it(self) -> None:
        source = (
            REPO_ROOT / "crates/tme-rules/src/content/validation/boundary/terms.rs"
        ).read_text(encoding="utf-8")
        self.assertIn("fn cargos_env_table_reaches_this_test_process", source)


if __name__ == "__main__":
    unittest.main()
