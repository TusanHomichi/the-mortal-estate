"""What the environment provides, asked once, answered with a reason.

A capability is the only sanctioned reason a step may not run. Everything else
is a pass or a failure. Keeping the list short and explicit is the point: when
a run reports INCOMPLETE, the reason names the missing capability and how to supply it.
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


def _probe_postgres_server(environ: Mapping[str, str]) -> tuple[bool, str]:
    """A PostgreSQL installation whose binaries can create and run a cluster."""
    search_path = environ.get("PATH")
    psql = shutil.which("psql", path=search_path)
    if psql is None:
        return False, "psql is not on PATH"
    bindir = Path(psql).resolve().parent
    # pg_config is discovery only: the binaries below are the requirement, so an answer
    # that is absent, failed or empty falls back to where psql itself lives.
    pg_config = shutil.which("pg_config", path=search_path)
    if pg_config is not None:
        try:
            configured = subprocess.run(
                [pg_config, "--bindir"], capture_output=True, text=True, timeout=60
            )
        except (OSError, subprocess.SubprocessError):
            configured = None
        if configured is not None and configured.returncode == 0:
            reported = (configured.stdout or "").strip()
            if reported:
                bindir = Path(reported)
    missing = [
        name for name in ("initdb", "pg_ctl", "pg_dump", "pg_restore")
        if not (bindir / name).is_file()
    ]
    if missing:
        return (
            False,
            f"{bindir} does not hold {', '.join(missing)}; a PostgreSQL server "
            "installation is required",
        )
    try:
        completed = subprocess.run(
            [str(bindir / "initdb"), "--version"], capture_output=True, text=True, timeout=60
        )
    except (OSError, subprocess.SubprocessError) as error:
        return False, f"{bindir}/initdb could not report its version: {error}"
    if completed.returncode != 0 or not (completed.stdout or "").strip():
        return False, f"{bindir}/initdb could not report its version"
    return True, f"PostgreSQL server binaries in {bindir}"


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


def _probe_feel_assets(environ: Mapping[str, str]) -> tuple[bool, str]:
    value = environ.get("TME_FEEL_ASSETS", "")
    if not value or not Path(value).is_absolute() or not Path(value).is_dir():
        return False, "TME_FEEL_ASSETS must name an absolute external candidate-packet directory"
    if Path(value).resolve().is_relative_to(ROOT):
        return False, "candidate assets must remain outside the checkout"
    return True, "external candidate-packet directory is available; browser proof validates its contents"


def _probe_browsers(environ: Mapping[str, str]) -> tuple[bool, str]:
    node = shutil.which("node", path=environ.get("PATH"))
    if node is None:
        return False, "Node is absent; install the browser toolchain first"
    script = "import { resolveProofBrowsers } from './proof/engines.mjs'; try { resolveProofBrowsers(); } catch (error) { console.error(error.message); process.exit(3); }"
    try:
        result = subprocess.run([node, "--input-type=module", "-e", script], cwd=ROOT / "web", env=dict(environ),
                                capture_output=True, text=True, timeout=15)
    except (OSError, subprocess.SubprocessError):
        return False, "browser binaries could not be checked"
    if result.returncode:
        return False, "install web dependencies and every Playwright engine in web/proof/engines.json; the capture proof also requires a working WebGL2 display"
    return True, "All rostered Playwright engines are installed; the producer verifies actual WebGL2 renderers"


FEEL_ASSETS = Capability("feel-assets", "an external candidate packet", _probe_feel_assets)
NODE = Capability("node", "Node 22 or newer and npm", _probe_node)
POSTGRES = Capability("postgres", "a PostgreSQL superuser URL and psql", _probe_postgres)
POSTGRES_SERVER = Capability(
    "postgres-server",
    "a PostgreSQL server installation: psql plus initdb, pg_ctl, pg_dump and pg_restore in one bin directory",
    _probe_postgres_server,
)
PRIVATE_TERMS_LIST = Capability(
    "private-terms", "the private banned-term denylist", _probe_private_terms
)

BROWSERS = Capability("browsers", "the complete browser proof roster", _probe_browsers)
CAPABILITIES: tuple[Capability, ...] = (NODE, POSTGRES, POSTGRES_SERVER, PRIVATE_TERMS_LIST, FEEL_ASSETS, BROWSERS)
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
