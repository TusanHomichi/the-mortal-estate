#!/usr/bin/env python3
"""Native first-expedition UI proof against its authored world and scratch PostgreSQL."""
from __future__ import annotations
import argparse
import json
from pathlib import Path
import subprocess
from live_server_harness import REPOSITORY_ROOT, World, read_admin_url, run
from run_browser_services_proof import BrowserFront, BrowserServer


class ExpeditionServer(BrowserServer):
    presentation_assets: Path

    def start_proxy(self, listen_port, upstream_port, certificate, key):
        return BrowserFront(self.run_directory, REPOSITORY_ROOT / "web/dist/play", listen_port,
                            upstream_port, certificate, key, self.presentation_assets)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--admin-url-file", required=True, type=Path)
    parser.add_argument("--assets", required=True, type=Path)
    parser.add_argument("--output", required=True, type=Path)
    parser.add_argument("--engine", choices=["chromium", "firefox", "webkit"])
    arguments = parser.parse_args()
    output = arguments.output.resolve()
    if output.is_relative_to(REPOSITORY_ROOT):
        parser.error("proof output must be outside the checkout")
    output.mkdir(parents=True, exist_ok=True)
    receipt = output / "verification.json"
    receipt.write_text(json.dumps({"verdict": "INCOMPLETE"}) + "\n")
    run(["npm", "--prefix", "web", "run", "build:play"])
    roster = json.loads((REPOSITORY_ROOT / "web/proof/engines.json").read_text())
    reports = []
    for engine in roster:
        if arguments.engine and engine != arguments.engine:
            continue
        server = ExpeditionServer(read_admin_url(arguments.admin_url_file),
                                  World.declared("content/lands/first-expedition/world.json"))
        server.presentation_assets = arguments.assets
        with server:
            config = dict(origin=server.origin, authority=str(server.authority), engine=engine,
                          username=server.username, password=server.password, output=str(output))
            result = subprocess.run(["node", "web/proof/expedition-proof.mjs"], cwd=REPOSITORY_ROOT,
                                    input=json.dumps(config), text=True, capture_output=True)
            if result.returncode:
                raise RuntimeError(f"{engine}: {result.stderr[-5000:]}")
            reports.append(json.loads((output / f"{engine}-expedition.json").read_text()))
            print(f"PASS {engine}/first-expedition", flush=True)
    receipt.write_text(json.dumps(dict(verdict="PASS", reports=reports), indent=2) + "\n")


if __name__ == "__main__":
    main()
