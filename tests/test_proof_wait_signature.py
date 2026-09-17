"""`eventually` waits for the timeout it is given, and callers pass two arguments.

`web/proof/death-return-proof.mjs` defines `eventually(predicate, timeout)` and
some call sites passed a third argument — `eventually(predicate, null, 60000)` —
left over from a differently shaped helper. JavaScript ignores extra arguments,
so the real timeout became `null`, `Date.now() + null` was the current time, and
the wait expired after one polling interval instead of a minute. On a slow engine
the predicate legitimately becomes true asynchronously, so runs failed for a
reason that had nothing to do with the game.

These cases execute the **shipped function**, extracted from the proof script
itself rather than reimplemented here, so a change to the definition is what the
test sees. They also scan every proof script for the three-argument call shape,
which is the defect that actually shipped: a caller cannot silently discard a
timeout it thinks it is passing.
"""

import json
import re
import subprocess
import tempfile
import textwrap
import unittest
from pathlib import Path

REPOSITORY_ROOT = Path(__file__).resolve().parents[1]
PROOF_DIRECTORY = REPOSITORY_ROOT / "web" / "proof"
PROOF_SCRIPTS = sorted(PROOF_DIRECTORY.glob("*.mjs"))

#: The shipped helper, as one self-contained function declaration.
EVENTUALLY = re.compile(r"async function eventually\([^)]*\)\{.*?\n\}", re.DOTALL)


def shipped_eventually(script: Path) -> str:
    source = script.read_text(encoding="utf-8")
    match = EVENTUALLY.search(source)
    if match is None:
        raise AssertionError(f"{script.name} no longer defines eventually")
    return match.group(0)


def run_node(source: str) -> dict:
    with tempfile.NamedTemporaryFile("w", suffix=".mjs", delete=False) as handle:
        handle.write(source)
        path = Path(handle.name)
    try:
        completed = subprocess.run(
            ["node", str(path)], capture_output=True, text=True, timeout=120, check=False
        )
    finally:
        path.unlink(missing_ok=True)
    if completed.returncode:
        raise AssertionError(f"node failed: {completed.stderr[-2000:]}")
    return json.loads(completed.stdout)


def eventually_call_arity_defects(scripts: list[Path]) -> list[str]:
    """Every `eventually(...)` call in `scripts` that passes more than two
    arguments.

    Parsed by hand rather than by a regular expression, because the arguments
    contain arrow functions whose bodies hold commas and parentheses of their
    own; that nesting is exactly where a naive split misreads the call.
    """
    defects = []
    for script in scripts:
        source = script.read_text(encoding="utf-8")
        if not EVENTUALLY.search(source):
            continue
        for match in re.finditer(r"\beventually\s*\(", source):
            depth = 1
            index = match.end()
            arguments, current = [], ""
            while index < len(source) and depth:
                character = source[index]
                if character in "([{":
                    depth += 1
                elif character in ")]}":
                    depth -= 1
                    if depth == 0:
                        break
                if character == "," and depth == 1:
                    arguments.append(current.strip())
                    current = ""
                else:
                    current += character
                index += 1
            if current.strip():
                arguments.append(current.strip())
            if len(arguments) > 2:
                defects.append(f"{script.name}: eventually({', '.join(arguments)})")
    return defects


class EventuallySignatureTests(unittest.TestCase):
    def test_no_proof_script_calls_eventually_with_a_third_argument(self):
        self.assertEqual(
            eventually_call_arity_defects(PROOF_SCRIPTS),
            [],
            "these calls pass more arguments than eventually accepts, so the "
            "timeout they name is silently dropped",
        )

    def test_the_shipped_helper_waits_for_an_asynchronous_predicate(self):
        helper = shipped_eventually(PROOF_DIRECTORY / "death-return-proof.mjs")
        program = textwrap.dedent(
            f"""
            const delay = ms => new Promise(resolve => setTimeout(resolve, ms));
            let stage = 'startup';
            {helper}
            let flag = false;
            const started = Date.now();
            setTimeout(() => {{ flag = true; }}, 250);
            await eventually(() => flag, 60000);
            // `Function.length` stops at the first defaulted parameter, so the
            // declaration's real parameter count is read from its source text.
            const declared = eventually.toString().match(/[(]([^)]*)[)]/)[1]
              .split(',').map(part => part.trim()).filter(Boolean);
            console.log(JSON.stringify({{elapsed: Date.now() - started, declared}}));
            """
        )
        observed = run_node(program)
        self.assertEqual(
            observed["declared"],
            ["predicate", "timeout=45000"],
            "eventually must declare exactly a predicate and a timeout",
        )
        self.assertGreaterEqual(
            observed["elapsed"],
            200,
            "the wait returned before the predicate became true",
        )

    def test_a_null_timeout_expires_immediately_which_is_why_arity_matters(self):
        # The failure mode itself, pinned so the reason for the test above stays
        # visible: a caller-supplied null reaches the arithmetic as zero.
        helper = shipped_eventually(PROOF_DIRECTORY / "death-return-proof.mjs")
        program = textwrap.dedent(
            f"""
            const delay = ms => new Promise(resolve => setTimeout(resolve, ms));
            let stage = 'shipping';
            {helper}
            let flag = false;
            setTimeout(() => {{ flag = true; }}, 250);
            const started = Date.now();
            let outcome = 'resolved';
            try {{ await eventually(() => flag, null); }} catch (error) {{ outcome = String(error.message); }}
            console.log(JSON.stringify({{elapsed: Date.now() - started, outcome}}));
            """
        )
        observed = run_node(program)
        self.assertLess(observed["elapsed"], 200, "a null timeout is not a wait")
        self.assertIn("Timed out: shipping", observed["outcome"])


if __name__ == "__main__":
    unittest.main()
