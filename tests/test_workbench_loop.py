"""The ordinary Workbench loop starts no processes.

Only the compiler bridge may spawn a program. Structural import checks and a
runtime subprocess tripwire protect selection, comments, staging, and retraction.
Godot retirement removed fresh capture; existing capture selection stays cheap.
"""

from __future__ import annotations

import ast
import json
import sys
import threading
import time
import unittest
import urllib.request
from http.server import ThreadingHTTPServer
from pathlib import Path

from workbench_test_support import TOOLS, StagedTree

from workbench import bridge, serve
from workbench.projection import DEFAULT_PROJECTION_PATH

PACKAGE = TOOLS / "workbench"

#: The compiler bridge is the only module allowed to start a program.
#: Everything else is the ordinary path.
BRIDGE_MODULE = "bridge.py"
EXPENSIVE_MODULES = (BRIDGE_MODULE,)

#: Anything that can start another program, directly or by proxy.
FORBIDDEN_IMPORTS = {
    "subprocess",
    "multiprocessing",
    "asyncio.subprocess",
    "pty",
    "ctypes",
    "shlex",
}
#: Calls that start a program. Named by their FULL dotted target rather than by
#: a bare attribute: this package now has an ordinary `run()` of its own — one
#: replay against a candidate — and a check that flagged every method called
#: `run` would have to be either wrong or weakened. Naming the owner instead
#: makes the check stricter, not looser: `subprocess.anything` is forbidden
#: outright, whatever it is called.
FORBIDDEN_CALL_OWNERS = {"subprocess", "multiprocessing"}
FORBIDDEN_CALL_NAMES = {
    "system",
    "popen",
    "execv",
    "execve",
    "execvp",
    "spawnl",
    "spawnv",
    "posix_spawn",
    "fork",
    "forkpty",
    "startfile",
}

#: Words that would mean a build, a test run, or a verification scope had found
#: its way onto the selection path.
FORBIDDEN_WORDS = ("cargo ", "cargo.", "pytest", "run_verification", "--workspace")

#: Milliseconds. Selection to resolved packet is a reading operation and must
#: stay one; the bound is generous so that a slow machine reports a real
#: regression rather than weather.
SELECTION_BUDGET_MSEC = 250.0

STANDARD_LIBRARY = set(sys.stdlib_module_names)


def modules() -> list[Path]:
    return sorted(PACKAGE.rglob("*.py"))


def selection_path_modules() -> list[Path]:
    return [path for path in modules() if path.name not in EXPENSIVE_MODULES]


def call_target(node: ast.Call) -> str:
    """The dotted name a call names, as far as it can be read statically."""
    parts: list[str] = []
    current = node.func
    while isinstance(current, ast.Attribute):
        parts.append(current.attr)
        current = current.value
    if isinstance(current, ast.Name):
        parts.append(current.id)
    return ".".join(reversed(parts))


def tree(path: Path) -> ast.Module:
    return ast.parse(path.read_text(encoding="utf-8"), filename=str(path))


DOCUMENTED = (ast.Module, ast.ClassDef, ast.FunctionDef, ast.AsyncFunctionDef)


def code_strings(node: ast.Module) -> set[str]:
    """Every string literal that is not a docstring — the ones that run.

    Docstrings are excluded by node identity rather than by value, because
    `ast.get_docstring` returns cleaned text that no longer equals the literal
    it came from, and comparing the two silently excludes nothing.
    """
    docstrings = set()
    for item in ast.walk(node):
        if not isinstance(item, DOCUMENTED):
            continue
        first = item.body[0] if item.body else None
        if (
            isinstance(first, ast.Expr)
            and isinstance(first.value, ast.Constant)
            and isinstance(first.value.value, str)
        ):
            docstrings.add(id(first.value))
    return {
        item.value
        for item in ast.walk(node)
        if isinstance(item, ast.Constant)
        and isinstance(item.value, str)
        and id(item) not in docstrings
    }


def imported_names(node: ast.Module) -> set[str]:
    """Every module a file imports, including the ones named after `from`.

    `from workbench import bridge` imports a module, not a symbol, and
    a reader that only recorded `workbench` would miss exactly the dependency
    this file exists to police.
    """
    names: set[str] = set()
    for item in ast.walk(node):
        if isinstance(item, ast.Import):
            names.update(alias.name for alias in item.names)
        elif isinstance(item, ast.ImportFrom) and item.level == 0 and item.module:
            names.add(item.module)
            names.update(f"{item.module}.{alias.name}" for alias in item.names)
        elif isinstance(item, ast.ImportFrom) and item.level > 0:
            names.update(f".{alias.name}" for alias in item.names)
    return names


class TheSelectionPathStartsNothing(unittest.TestCase):
    def test_the_package_has_modules_to_inspect(self) -> None:
        """A structural proof over an empty set proves nothing."""
        self.assertGreaterEqual(len(modules()), 8)
        for name in EXPENSIVE_MODULES:
            self.assertIn(name, {path.name for path in modules()})

    def test_no_selection_path_module_imports_a_way_to_start_a_process(self) -> None:
        for path in selection_path_modules():
            with self.subTest(module=path.name):
                names = imported_names(tree(path))
                self.assertEqual(names & FORBIDDEN_IMPORTS, set())
                self.assertNotIn("os.system", names)

    def test_no_selection_path_module_calls_a_way_to_start_a_process(self) -> None:
        for path in selection_path_modules():
            with self.subTest(module=path.name):
                for node in ast.walk(tree(path)):
                    if not isinstance(node, ast.Call):
                        continue
                    target = call_target(node)
                    parts = target.split(".")
                    self.assertNotIn(
                        parts[0],
                        FORBIDDEN_CALL_OWNERS,
                        f"{path.name} calls {target}() on the ordinary path",
                    )
                    self.assertNotIn(
                        parts[-1],
                        FORBIDDEN_CALL_NAMES,
                        f"{path.name} calls {target}() on the ordinary path",
                    )

    def test_no_module_names_a_build_or_verification_command_in_code(self) -> None:
        """The rebuild command may be QUOTED in a message. It may not be run."""
        for path in modules():
            with self.subTest(module=path.name):
                for word in FORBIDDEN_WORDS:
                    offenders = [text for text in code_strings(tree(path)) if word in text]
                    # The one permitted occurrence is the honest repair
                    # instruction shown when the projection is missing.
                    offenders = [
                        text for text in offenders if "cargo run -p tme-authoring" not in text
                    ]
                    self.assertEqual(offenders, [], f"{path.name} names {word!r}")

    def test_the_package_depends_on_the_standard_library_alone(self) -> None:
        """No external dependency, so `python3 serve.py` is the whole install."""
        for path in modules():
            with self.subTest(module=path.name):
                for name in imported_names(tree(path)):
                    root = name.split(".")[0]
                    if root in ("workbench", ""):
                        continue
                    self.assertIn(
                        root,
                        STANDARD_LIBRARY,
                        f"{path.name} imports {name}, which is not in the standard library",
                    )

    def test_the_view_reads_the_compilers_projection_rather_than_the_authored_source(
        self,
    ) -> None:
        """No second parse of the master, so no second geography authority."""
        for path in modules():
            with self.subTest(module=path.name):
                # Prose may DISCUSS the authored document; no code may open one.
                for text in code_strings(tree(path)):
                    self.assertNotIn(".tmj", text, f"{path.name} names an authored member")


class TheExpensiveModulesAreNamedAndBounded(unittest.TestCase):
    def test_exactly_two_modules_can_start_a_program(self) -> None:
        """Two, named, and no third arriving quietly.

        The list is asserted whole rather than as a membership test, so adding a
        third module that shells out turns this red rather than passing because
        the two that were checked are still there.
        """
        starters = sorted(
            path.name for path in modules() if "subprocess" in imported_names(tree(path))
        )
        self.assertEqual(starters, sorted(EXPENSIVE_MODULES))

    def test_the_bridge_starts_the_compiler_and_nothing_else(self) -> None:
        """The whole command, argument by argument, with no shell anywhere in it."""
        from workbench import bridge

        command = list(bridge.COMPILER_COMMAND)
        self.assertEqual(command[0], "cargo")
        self.assertIn("-p", command)
        self.assertEqual(command[command.index("-p") + 1], "tme-authoring")
        self.assertEqual(command[-1], "--")
        for word in ("sh", "bash", "-c", "eval", "xvfb-run"):
            self.assertNotIn(word, command, f"the compiler command names {word!r}")

    def test_the_bridge_is_reached_only_by_the_modules_that_own_a_verdict(self) -> None:
        """A verdict costs a program, so who may ask for one is a bounded list."""
        importers = sorted(
            path.name
            for path in modules()
            if path.name != BRIDGE_MODULE
            and any(name.endswith("bridge") for name in imported_names(tree(path)))
        )
        self.assertEqual(importers, ["apply.py", "replay.py", "serve.py", "stage.py"])

    def test_the_bridge_never_writes_tracked_content(self) -> None:
        """The compiler's one tracked-writing mode is not reachable from here.

        `cargo run -p tme-authoring` with no subcommand writes the tracked
        projection. Every call this module makes names a subcommand, and none of
        them is that one.
        """
        from workbench import bridge

        source = (PACKAGE / BRIDGE_MODULE).read_text(encoding="utf-8")
        for subcommand in ("describe-operations", "validate-candidate", "project-candidate", "replay"):
            self.assertIn(f'"{subcommand}"', source)
        for forbidden in ("--check", "--report"):
            self.assertNotIn(f'"{forbidden}"', source)
        self.assertEqual(bridge.COMPILER_COMMAND[-1], "--")



class TheOrdinaryRoutesTouchNothing(StagedTree):
    """The behavioural half: drive the real server and watch the tripwire."""

    def setUp(self) -> None:
        super().setUp()
        self.started: list[list[str]] = []

        def tripwire(command, **_keywords):
            self.started.append(list(command))
            raise AssertionError("an ordinary route started a program")

        for module in (bridge,):
            original = module.subprocess.run
            module.subprocess.run = tripwire
            self.addCleanup(setattr, module.subprocess, "run", original)

        workbench = serve.Workbench(self.staged, DEFAULT_PROJECTION_PATH, "session-loop")
        handler = type(
            "BoundHandler",
            (serve.Handler,),
            {"workbench": workbench, "log_message": lambda *_a, **_k: None},
        )
        self.server = ThreadingHTTPServer(("127.0.0.1", 0), handler)
        self.server.daemon_threads = True
        self.addCleanup(self.server.server_close)
        threading.Thread(target=self.server.serve_forever, daemon=True).start()
        self.addCleanup(self.server.shutdown)
        host, port = self.server.server_address[:2]
        self.base = f"http://{host}:{port}"

    def get(self, path: str) -> dict:
        with urllib.request.urlopen(self.base + path, timeout=10) as response:
            return json.loads(response.read())

    def post(self, path: str, body: dict) -> dict:
        request = urllib.request.Request(
            self.base + path,
            data=json.dumps(body).encode("utf-8"),
            headers={"Content-Type": "application/json"},
            method="POST",
        )
        with urllib.request.urlopen(request, timeout=10) as response:
            return json.loads(response.read())

    def test_retired_fresh_capture_route_is_refused(self) -> None:
        with self.assertRaises(urllib.error.HTTPError) as caught:
            self.post("/api/capture", {})
        self.addCleanup(caught.exception.close)
        self.assertEqual(caught.exception.code, 404)

    def test_no_ordinary_route_starts_a_program(self) -> None:
        self.get("/api/state")
        self.get("/api/projection")
        self.post("/api/preview", {"member": "surface", "gesture": "click", "cell": {"x": 8, "y": 6}})
        packet = self.post(
            "/api/selection",
            {"member": "surface", "gesture": "box", "rect": {"x": 8, "y": 6, "width": 2, "height": 2}},
        )["packet"]
        self.get(f"/api/packet?id={packet['selection_id']}")
        self.post("/api/comment", {"selection_id": packet["selection_id"], "comment": "nothing ran"})
        # V1's staging routes are on the ordinary path too: proposing an edit is
        # a file append, and only asking what the edit MEANS costs a program.
        staged = self.post(
            "/api/stage",
            {
                "selection_id": packet["selection_id"],
                "verb": "set_terrain",
                "parameters": {"cells": [{"x": 8, "y": 6}], "class": "testland_grass"},
            },
        )["record"]
        self.get("/api/state")
        self.post("/api/retract", {"record_id": staged["record_id"], "reason": "nothing ran"})
        self.assertEqual(self.started, [])

    def test_selection_to_written_packet_stays_in_milliseconds(self) -> None:
        """Criterion 7's guaranteed half. The measured half is the capture."""
        body = {"member": "surface", "gesture": "click", "cell": {"x": 12, "y": 14}}
        self.post("/api/preview", dict(body))  # warm the loader once
        elapsed = []
        for _ in range(5):
            started = time.monotonic()
            self.post("/api/selection", dict(body))
            elapsed.append((time.monotonic() - started) * 1000.0)
        worst = max(elapsed)
        self.assertLess(
            worst,
            SELECTION_BUDGET_MSEC,
            f"the slowest selection took {worst:.1f} ms",
        )
        self.assertEqual(self.started, [])


class TheBrowserSideStartsNothingEither(unittest.TestCase):
    def test_the_page_loads_no_external_resource(self) -> None:
        """Loopback only, and offline: nothing here reaches past the checkout.

        Every file the app directory holds, rather than three named ones. The
        page is a graph of ES modules now, and a hand-written list would have
        quietly stopped covering the modules added after it was written — the
        assertion would still pass while testing less of the page each time.
        """
        assets = sorted(path for path in (PACKAGE / "app").iterdir() if path.is_file())
        self.assertTrue(assets, "the app directory holds no files")
        for asset in assets:
            with self.subTest(asset=asset.name):
                text = asset.read_text(encoding="utf-8")
                self.assertNotIn("http://", text.replace("http://127.0.0.1", ""))
                self.assertNotIn("https://", text)
                self.assertNotIn("<script src=\"//", text)

    def test_the_page_labels_which_view_the_owner_is_looking_at(self) -> None:
        """The owner must never mistake either window for the game."""
        markup = (PACKAGE / "app" / "index.html").read_text(encoding="utf-8")
        self.assertIn("LOGICAL VIEW", markup)
        self.assertIn("not a gameplay preview", markup)
        self.assertIn("CAPTURE VIEW", markup)
        self.assertIn("real client frame", markup)

    def test_the_page_asks_the_server_what_a_gesture_means(self) -> None:
        """Agent parity is a law: one resolver, and it is not in the browser.

        Read over every module the page loads, concatenated: the entry point is
        one file of several, and a resolver invented in any of them would be the
        same second answer.
        """
        scripts = sorted((PACKAGE / "app").glob("*.js"))
        self.assertTrue(scripts, "the app directory holds no modules")
        script = "\n".join(path.read_text(encoding="utf-8") for path in scripts)
        self.assertIn("/api/capture/preview", script)
        self.assertIn("/api/capture/selection", script)
        for invented in ("function resolveIdentities", "computeSemantic", "identity_raster"):
            self.assertNotIn(invented, script)


if __name__ == "__main__":
    unittest.main()
