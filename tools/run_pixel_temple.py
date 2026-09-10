#!/usr/bin/env python3
"""Build the world product and serve its temple on a disposable local authority."""
from __future__ import annotations

import argparse
from dataclasses import replace
import json
import os
from pathlib import Path
import signal
import subprocess

from live_server_harness import REPOSITORY_ROOT, World, read_admin_url, run
from run_browser_services_proof import BrowserFront, BrowserServer


def temple_world(exterior: bool = False) -> World:
    world = World.declared("content/lands/first-expedition/world.json", key="pixel-temple-study")
    seed = json.loads((REPOSITORY_ROOT / world.simulation_seed).read_text())
    player = next(actor for actor in seed["actors"] if actor["id"] == world.controlled_actor)
    player["location"] = dict(realm="first_expedition", level="arrival" if exterior else "temple",
                              position=dict(x=13, y=9) if exterior else dict(x=3, y=6))
    return replace(world, simulation_seed=None, generated_seed=seed)


class PixelServer(BrowserServer):
    bundle: Path
    assets: Path

    def start_proxy(self, listen_port, upstream_port, certificate, key):
        return BrowserFront(self.run_directory, self.bundle, listen_port, upstream_port,
                            certificate, key, self.assets)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--admin-url-file", required=True, type=Path)
    parser.add_argument("--assets", required=True, type=Path)
    parser.add_argument("--output", required=True, type=Path)
    parser.add_argument("--public-origin", help="owner-authorized HTTPS reverse-proxy origin")
    parser.add_argument("--proof", action="store_true", help="exercise all browser engines, then stop")
    parser.add_argument("--engine", choices=["chromium", "firefox", "webkit"])
    parser.add_argument("--exterior", action="store_true", help="start at the temple frontage and run exterior proof")
    parser.add_argument("--entry", action="store_true", help="prove game entry and durable creation on a disposable authority")
    parser.add_argument("--dungeon", action="store_true", help="inspect raised dungeon scenery on a disposable authority")
    parser.add_argument("--profile", action="store_true", help="measure steady native rendering instead of the walkthrough")
    args = parser.parse_args()
    if args.dungeon and (not args.proof or args.entry or args.profile or args.exterior):
        parser.error("--dungeon requires --proof and cannot combine with other proof modes")
    if args.entry and (not args.proof or args.profile or args.exterior):
        parser.error("--entry requires --proof and cannot combine with --profile or --exterior")
    if args.profile and not args.proof:
        parser.error("--profile requires --proof")
    if args.profile and args.exterior:
        parser.error("--profile starts in the temple and measures both scenes; omit --exterior")
    if args.public_origin and args.proof:
        parser.error("remote preview routing is separate from disposable --proof runs")
    output = args.output.resolve()
    assets = args.assets.resolve(strict=True)
    if output.is_relative_to(REPOSITORY_ROOT) or assets.is_relative_to(REPOSITORY_ROOT):
        parser.error("study output and assets must be outside the checkout")
    if not assets.is_dir() or not (assets / "pixel-manifest.json").is_file():
        parser.error("assets must name a complete pixel packet")
    output.mkdir(parents=True, exist_ok=True)
    run(["node", "web/proof/build-play.mjs", str(output / "bundle"), "world"])
    engines = json.loads((REPOSITORY_ROOT / "web/proof/engines.json").read_text()) if args.proof else [None]
    if args.engine:
        if not args.proof:
            parser.error("--engine requires --proof")
        engines = [args.engine]
    receipt = output / "verification.json"
    if args.proof:
        receipt.write_text(json.dumps(dict(verdict="INCOMPLETE")) + "\n")
    reports = []
    for engine in engines:
        world = temple_world(args.exterior)
        if args.dungeon:
            player = next(actor for actor in world.generated_seed["actors"] if actor["id"] == world.controlled_actor)
            player["location"] = dict(realm="first_expedition", level="d1_entry", position=dict(x=24, y=7))
        server = PixelServer(read_admin_url(args.admin_url_file), world, public_origin=args.public_origin)
        server.bundle = output / "bundle"
        server.assets = assets
        with server:
            config = dict(origin=server.origin, local_origin=server.local_origin,
                          authority=str(server.authority), engine=engine,
                          username=server.username, password=server.password, output=str(output), scenario="doors" if args.dungeon else None)
            if args.proof:
                proof = "dungeon" if args.dungeon else "entry-creation" if args.entry else "pixel-performance" if args.profile else "pixel-exterior" if args.exterior else "pixel-temple"
                result = subprocess.run(["node", f"web/proof/{proof}-proof.mjs"], cwd=REPOSITORY_ROOT,
                                        input=json.dumps(config), text=True, capture_output=True,
                                        env={**os.environ, "NODE_EXTRA_CA_CERTS": str(server.authority)})
                if result.returncode:
                    raise RuntimeError(f"{engine}: {result.stderr[-5000:]}")
                reports.append(json.loads((output / f"{engine}-{proof}.json").read_text()))
                print(f"PASS {engine}/{proof}", flush=True)
            else:
                access = output / "access.json"
                descriptor = os.open(access, os.O_WRONLY | os.O_CREAT | os.O_TRUNC, 0o600)
                os.fchmod(descriptor, 0o600)
                with os.fdopen(descriptor, "w") as stream:
                    json.dump(config, stream)
                print(f"Pixel temple: {server.origin}/play.html", flush=True)
                print(f"Local access details: {access}. Stop with Ctrl-C; the scratch world is disposable.", flush=True)
                try:
                    signal.signal(signal.SIGTERM, lambda *_: (_ for _ in ()).throw(KeyboardInterrupt()))
                    signal.pause()
                except KeyboardInterrupt:
                    pass
                finally:
                    access.unlink(missing_ok=True)
    if args.proof:
        receipt.write_text(json.dumps(dict(verdict="PASS" if len(reports) == 3 else "INSPECTION", reports=reports), indent=2) + "\n")


if __name__ == "__main__":
    main()
