#!/usr/bin/env python3
"""Manage a dedicated, private TME development server on a shared host."""
from __future__ import annotations

import argparse
import fcntl
import json
import os
import sys
from pathlib import Path

from common import Installation, UNITS, run
from operations import activate, backup, health, restore, restore_drill
from provision import install, stage_release, tls


def main():
    os.umask(0o077)
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", type=Path, default=Path.home() / ".local/share/tme-development")
    commands = parser.add_subparsers(dest="command", required=True)
    setup = commands.add_parser("install")
    setup.add_argument("--configuration", type=Path, required=True)
    setup.add_argument("--denylist", type=Path, required=True)
    setup.add_argument("--postgres-bin", type=Path, default=Path("/usr/lib/postgresql/18/bin"))
    for name in ("start", "stop", "restart", "status", "logs", "backup", "stage", "renew-tls"):
        commands.add_parser(name)
    drill = commands.add_parser("restore-drill")
    drill.add_argument("backup", type=Path)
    recovery = commands.add_parser("restore")
    recovery.add_argument("backup", type=Path)
    recovery.add_argument("--replace-development-world", action="store_true", required=True)
    deploy = commands.add_parser("activate")
    deploy.add_argument("release", type=Path)
    proof = commands.add_parser("proof")
    proof.add_argument("--restart", action="store_true", help="also restart the installed services during an action")
    browser = commands.add_parser("browser-proof")
    browser.add_argument("--output", type=Path, required=True)
    remove = commands.add_parser("uninstall")
    remove.add_argument("--purge-private-development-data", action="store_true")
    arguments = parser.parse_args()
    site = Installation(arguments.root)
    site.root.parent.mkdir(parents=True, exist_ok=True)
    # The lock is outside the root so installation cannot race its own creation.
    with (site.root.parent / ("." + site.root.name + ".lock")).open("a") as lock:
        fcntl.flock(lock, fcntl.LOCK_EX | fcntl.LOCK_NB)
        if arguments.command == "install":
            install(site, arguments.configuration.resolve(), arguments.denylist.resolve(), arguments.postgres_bin.resolve())
            result = health(site)
        elif site.settings is None:
            raise RuntimeError("private development installation is absent")
        elif arguments.command == "stop":
            result = site.service("stop", *reversed(UNITS))
        elif arguments.command in ("start", "restart"):
            site.check_release()
            if arguments.command == "restart":
                site.service("stop", *reversed(UNITS))
            site.service("start", *UNITS)
            result = health(site)
        elif arguments.command == "status":
            result = health(site, timeout=1)
        elif arguments.command == "logs":
            result = run(["journalctl", "--user", "--no-pager", "-n", "80", *[f"--unit={name}" for name in UNITS]])
        elif arguments.command == "backup":
            result = str(backup(site))
        elif arguments.command == "restore-drill":
            result = restore_drill(site, arguments.backup)
        elif arguments.command == "restore":
            result = restore(site, arguments.backup)
        elif arguments.command == "stage":
            result = str(stage_release(site))
        elif arguments.command == "activate":
            result = activate(site, arguments.release)
        elif arguments.command == "renew-tls":
            tls(site)
            site.service("restart", UNITS[2])
            result = health(site)
        elif arguments.command == "proof":
            from proof import prove
            result = prove(site, restart=arguments.restart)
        elif arguments.command == "browser-proof":
            from common import REPO
            health(site)
            accounts = json.loads((site.config / "test-accounts.json").read_text())
            result = run(["node", REPO / "web/proof/play-proof.mjs"], input=json.dumps({
                "origin": site.origin, "authority": str(site.config / "tls/ca.pem"),
                "accounts": accounts, "output": str(arguments.output.resolve()),
            }), cwd=REPO, timeout=300)
        elif arguments.command == "uninstall":
            files = [site.units / (name + ".service") for name in UNITS]
            if any(path.exists() and str(site.root) not in path.read_text() for path in files):
                raise RuntimeError("a service unit belongs to another installation")
            names = [name for name, path in zip(UNITS, files) if path.exists()]
            if names:
                site.service("stop", *reversed(names))
                site.service("disable", *names)
            for path in files:
                path.unlink(missing_ok=True)
            run(["systemctl", "--user", "daemon-reload"])
            if arguments.purge_private_development_data:
                import shutil
                shutil.rmtree(site.root)
            result = "private services removed; " + ("private data purged" if arguments.purge_private_development_data else "data retained")
        print(json.dumps(result, indent=2) if not isinstance(result, str) else result)
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except (OSError, ValueError, RuntimeError) as error:
        print(f"development operation refused: {error}", file=sys.stderr)
        raise SystemExit(1)
