#!/usr/bin/env python3
"""Fail when a banned term appears in this repository's carried files.

What this defends
-----------------
The predecessor project shipped its source-lineage denylist as a literal array
inside public code, which named the very lineage the denylist existed to keep
out of public surfaces. This check inverts that: **the mechanism is public, the
terms are private.** The term list loads from a data file the public cut does
not carry (`.boundary/banned-terms.txt`, git-ignored), and the tracked tests
prove the mechanism against invented nonsense terms in
`tests/fixtures/synthetic-terms.txt`.

Fail-closed semantics
---------------------
A missing, unreadable, non-UTF-8, or entry-less term file exits 3 with a
distinct message. It never skips and never passes. A boundary check that goes
quiet when its input disappears is the exact false-green class the authoring
contract exists to kill.

What is scanned
---------------
Every carried file (see boundary_common): both its **path** and, when the file
is text, its **contents**. Binary files are detected by a null-byte sniff;
their contents are skipped but their paths are still checked, because a
filename carries meaning no matter what the bytes are. The term file itself is
excluded from the scan so that pointing the check at a tracked fixture of terms
cannot make that fixture indict itself.

Matching rule
-------------
Case-insensitive, with word-ish boundaries: a match must not be flanked by an
alphanumeric character, so a short term never fires from inside a longer word.
Inside a term, any run of whitespace or punctuation matches any run of
non-alphanumeric characters *including none* — a two-word term such as
"flanquil brindlewisp" also catches "Flanquil.Brindlewisp",
"flanquil_brindlewisp", and "FlanquilBrindlewisp". The rule deliberately errs
toward catching: a false positive costs a human ten seconds of review, and a
false negative ships.

The examples in this file are invented nonsense on purpose. No real banned term
appears in any carried file — that is the point of the whole arrangement, and
this docstring is inside the scan's own scope.
"""

from __future__ import annotations

import argparse
import re
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))

from boundary_common import (  # noqa: E402
    ConfigError,
    Finding,
    carried_files,
    fail_closed,
    load_list_file,
    read_text,
    report,
    resolve_root,
)

CHECK_NAME = "banned-terms"
DEFAULT_TERMS_PATH = ".boundary/banned-terms.txt"


def compile_terms(terms: list[str]) -> list[tuple[str, re.Pattern[str]]]:
    """Compile each term into its word-ish, separator-tolerant pattern."""
    compiled = []
    for term in terms:
        parts = [part for part in re.split(r"[^0-9A-Za-z]+", term) if part]
        if not parts:
            raise ConfigError(f"term has no alphanumeric content: {term!r}")
        body = r"[^0-9A-Za-z]*".join(re.escape(part) for part in parts)
        pattern = re.compile(
            rf"(?<![0-9A-Za-z]){body}(?![0-9A-Za-z])",
            re.IGNORECASE,
        )
        compiled.append((term, pattern))
    return compiled


def scan(root: Path, terms_path: Path) -> list[Finding]:
    terms = load_list_file(terms_path, "banned-terms")
    patterns = compile_terms(terms)

    try:
        excluded = terms_path.resolve().relative_to(root.resolve()).as_posix()
    except ValueError:
        excluded = None

    findings: list[Finding] = []
    for relative in carried_files(root):
        if relative == excluded:
            continue
        for term, pattern in patterns:
            if pattern.search(relative):
                findings.append(
                    Finding(relative, f"banned term {term!r} in the file path")
                )
        text = read_text(root / relative)
        if text is None:
            continue
        for number, line in enumerate(text.splitlines(), start=1):
            for term, pattern in patterns:
                if pattern.search(line):
                    findings.append(
                        Finding(relative, f"banned term {term!r}", line=number)
                    )
    return findings


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--root", default=None, help="repository root to scan")
    parser.add_argument(
        "--terms",
        default=None,
        help=f"term data file (default: <root>/{DEFAULT_TERMS_PATH})",
    )
    arguments = parser.parse_args(argv)

    try:
        root = resolve_root(arguments.root)
        terms_path = (
            Path(arguments.terms).resolve()
            if arguments.terms
            else root / DEFAULT_TERMS_PATH
        )
        findings = scan(root, terms_path)
    except ConfigError as error:
        return fail_closed(CHECK_NAME, error)
    return report(CHECK_NAME, findings)


if __name__ == "__main__":
    sys.exit(main())
