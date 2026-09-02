"""What the environment provides, asked once, answered with a reason.

A capability is the only sanctioned reason a step may not run. Everything else
is a pass or a failure. Keeping the list short and explicit is the point: when
a run reports INCOMPLETE, the reason is one of these four sentences, and each
one says how to supply what is missing.
"""

from __future__ import annotations

import os
import re
import shutil
import subprocess
import sys
from pathlib import Path
from typing import Mapping

from .model import Capability

TOOLS = Path(__file__).resolve().parents[1]
if str(TOOLS) not in sys.path:
    sys.path.insert(0, str(TOOLS))

from boundary_common import PRIVATE_TERMS_RELATIVE, private_terms_path  # noqa: E402

ROOT = Path(__file__).resolve().parents[2]

#: The client binary this repository is pinned to. Checked by asking the
#: binary, never by trusting a path name.
GODOT_VERSION = "4.7.2.stable.official.ed1daf0bf"

#: The private denylist. Absent on every clean clone by design; see
#: docs/boundary-checks.md, "The private-terms convention". The relative path
#: is owned by boundary_common so the check and this probe cannot disagree.
PRIVATE_TERMS = PRIVATE_TERMS_RELATIVE

#: The tracked nonsense-word fixture the banned-terms mechanism degrades onto.
SYNTHETIC_TERMS = "tools/ci-synthetic-banned-terms.txt"

#: The file naming a PostgreSQL superuser URL, used only to create and drop
#: scratch databases. Never a value in the environment: a URL with a password
#: in it does not belong in a process listing.
ADMIN_URL_VARIABLE = "TME_PG_ADMIN_URL_FILE"

GODOT_VARIABLE = "TME_GODOT"


def _probe_node(environ: Mapping[str, str]) -> tuple[bool, str]:
    search_path = environ.get("PATH")
    node = shutil.which("node", path=search_path)
    npm = shutil.which("npm", path=search_path)
    if node is None:
        return False, "node is not on PATH; the browser client requires Node 22 or newer"
    if npm is None:
        return False, "npm is not on PATH; it is required for the browser client"
    try:
        completed = subprocess.run(
            [node, "--version"], capture_output=True, text=True, timeout=60
        )
        npm_completed = subprocess.run(
            [npm, "--version"], capture_output=True, text=True, timeout=60
        )
    except (OSError, subprocess.SubprocessError) as error:
        return False, f"node or npm could not be asked for its version: {error}"
    reported = (completed.stdout or "").strip()
    match = re.fullmatch(r"v(\d+)(?:\.\d+){2}", reported)
    if completed.returncode != 0 or match is None:
        return False, f"{node} reports an invalid version {reported!r}"
    if int(match.group(1)) < 22:
        return False, f"{node} reports {reported}; the browser client requires Node 22 or newer"
    npm_reported = (npm_completed.stdout or "").strip()
    if npm_completed.returncode != 0 or not npm_reported:
        return False, f"{npm} could not report a usable version"
    return True, f"node {reported}, npm {npm_reported}"


def _probe_godot(environ: Mapping[str, str]) -> tuple[bool, str]:
    value = environ.get(GODOT_VARIABLE, "")
    if not value:
        return False, f"{GODOT_VARIABLE} is not set; it must name the pinned client binary"
    binary = Path(value)
    if not binary.is_file() or not os.access(binary, os.X_OK):
        return False, f"{GODOT_VARIABLE}={value} does not name an executable file"
    try:
        completed = subprocess.run(
            [str(binary), "--version"], capture_output=True, text=True, timeout=60
        )
    except (OSError, subprocess.SubprocessError) as error:
        return False, f"{binary} could not be asked for its version: {error}"
    reported = (completed.stdout or "").strip().splitlines()
    reported = reported[-1].strip() if reported else ""
    if reported != GODOT_VERSION:
        return False, f"{binary} reports {reported!r}; this tree is pinned to {GODOT_VERSION}"
    return True, f"{binary} is {GODOT_VERSION}"


def _probe_postgres(environ: Mapping[str, str]) -> tuple[bool, str]:
    value = environ.get(ADMIN_URL_VARIABLE, "")
    if not value:
        return (
            False,
            f"{ADMIN_URL_VARIABLE} is not set; it must name a readable file holding a "
            "PostgreSQL superuser URL used only to create and drop scratch databases",
        )
    path = Path(value)
    if not path.is_file() or not os.access(path, os.R_OK):
        return False, f"{ADMIN_URL_VARIABLE}={value} does not name a readable file"
    if shutil.which("psql") is None:
        return False, "psql is not on PATH"
    if not path.read_text(encoding="utf-8").strip():
        return False, f"{value} is empty"
    return True, f"superuser URL from {value}, psql present"


def _probe_private_terms(_environ: Mapping[str, str]) -> tuple[bool, str]:
    path = private_terms_path(ROOT)
    if not path.is_file() or not os.access(path, os.R_OK):
        return (
            False,
            f"{PRIVATE_TERMS} is absent; the banned-terms mechanism will run against the "
            f"tracked synthetic fixture and asserts nothing about the real denylist",
        )
    if path == ROOT / PRIVATE_TERMS:
        return True, f"{PRIVATE_TERMS} is present"
    return True, f"{PRIVATE_TERMS} is present in this linked worktree's main checkout ({path})"


def _probe_display(environ: Mapping[str, str]) -> tuple[bool, str]:
    if environ.get("DISPLAY"):
        return True, f"DISPLAY={environ['DISPLAY']}"
    if shutil.which("xvfb-run") is not None:
        return True, "no DISPLAY, but xvfb-run is available"
    return False, "no DISPLAY and no xvfb-run; a windowed capture cannot be taken"


GODOT = Capability("godot", "the pinned Godot client binary", _probe_godot)
NODE = Capability("node", "Node 22 or newer and npm", _probe_node)
POSTGRES = Capability("postgres", "a PostgreSQL superuser URL and psql", _probe_postgres)
PRIVATE_TERMS_LIST = Capability(
    "private-terms", "the private banned-term denylist", _probe_private_terms
)
DISPLAY = Capability("display", "a display a windowed capture can use", _probe_display)

CAPABILITIES: tuple[Capability, ...] = (GODOT, NODE, POSTGRES, PRIVATE_TERMS_LIST, DISPLAY)
BY_NAME = {capability.name: capability for capability in CAPABILITIES}


def evaluate_all(environ: Mapping[str, str]) -> dict[str, "object"]:
    """Probe every capability once. Steps read the answers; nothing re-probes."""
    return {name: capability.evaluate(environ) for name, capability in BY_NAME.items()}


def _probe_capture_output(environ: Mapping[str, str]) -> tuple[bool, str]:
    value = environ.get("TME_CAPTURE_OUTPUT", "")
    if not value:
        return False, "TME_CAPTURE_OUTPUT is not set; it must name a directory to write captures into"
    if not Path(value).is_dir():
        return False, f"TME_CAPTURE_OUTPUT={value} does not name an existing directory"
    return True, f"captures write to {value}"


CAPTURE_OUTPUT = Capability(
    "capture-output", "a directory to write owner-invoked captures into", _probe_capture_output
)

CAPABILITIES = (*CAPABILITIES, CAPTURE_OUTPUT)
BY_NAME[CAPTURE_OUTPUT.name] = CAPTURE_OUTPUT
