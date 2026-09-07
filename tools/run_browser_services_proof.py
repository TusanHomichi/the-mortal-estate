#!/usr/bin/env python3
"""Real browser service actions against scratch PostgreSQL and carried fixtures.

The client is freshly built. No intercepted requests, injected commands or
private town assets are used. Fixtures establish integration, not fidelity.
"""
from __future__ import annotations

import argparse
import json
import os
from pathlib import Path
import subprocess

from live_server_harness import LiveServer, REPOSITORY_ROOT, World, read_admin_url, run


def quoted(path: Path) -> str:
    return '"' + str(path).replace('\\', '\\\\').replace('"', '\\"').replace('$', '\\$') + '"'


class BrowserFront:
    def __init__(self, directory: Path, bundle: Path, listen: int, upstream: int, certificate: Path, key: Path, presentation_assets: Path | None = None):
        asset_location = ""
        if presentation_assets is not None:
            assets = presentation_assets.resolve(strict=True)
            if not assets.is_dir() or assets.is_relative_to(REPOSITORY_ROOT):
                raise ValueError("presentation study assets must be an external directory")
            asset_alias = quoted(assets)[:-1] + '/"'
            asset_location = f"location /feel-assets/ {{ alias {asset_alias}; }}"
        configuration = directory / "browser-nginx.conf"
        configuration.write_text(f"""worker_processes 1;
pid {quoted(directory / 'nginx.pid')};
error_log stderr warn;
events {{ worker_connections 128; }}
http {{
  include /etc/nginx/mime.types;
  default_type application/octet-stream;
  access_log off;
  client_body_temp_path {quoted(directory / 'client-temp')};
  proxy_temp_path {quoted(directory / 'proxy-temp')};
  server {{
    listen 127.0.0.1:{listen} ssl;
    server_name localhost;
    ssl_certificate {quoted(certificate)};
    ssl_certificate_key {quoted(key)};
    root {quoted(bundle)};
    index play.html;
    add_header Cache-Control no-store always;
    location /v4/ {{
      proxy_pass http://127.0.0.1:{upstream};
      proxy_http_version 1.1;
      proxy_set_header Host $http_host;
      proxy_set_header Upgrade $http_upgrade;
      proxy_set_header Connection "upgrade";
      proxy_buffering off;
      proxy_read_timeout 120s;
    }}
    {asset_location}
    location / {{ try_files $uri $uri/ =404; }}
  }}
}}
""", encoding="utf-8")
        self.log = (directory / "browser-front.log").open("w", encoding="utf-8")
        try:
            self.process = subprocess.Popen(["/usr/sbin/nginx", "-p", str(directory), "-c", str(configuration),
                                             "-g", "daemon off;"], stdout=self.log, stderr=subprocess.STDOUT)
        except BaseException:
            self.log.close()
            raise

    def close(self):
        if self.process.poll() is None:
            self.process.terminate()
            try:
                self.process.wait(timeout=10)
            except subprocess.TimeoutExpired:
                self.process.kill()
                self.process.wait(timeout=10)
        self.log.close()


class BrowserServer(LiveServer):
    def start_proxy(self, listen_port, upstream_port, certificate, key):
        return BrowserFront(self.run_directory, REPOSITORY_ROOT / "web/dist/play",
                            listen_port, upstream_port, certificate, key)


def fixture_world(name: str) -> World:
    source = REPOSITORY_ROOT / "content/test-corpus" / f"{name}.json"
    document = json.loads(source.read_text())
    def resolve(key):
        return str((source.parent / document[key]).resolve().relative_to(REPOSITORY_ROOT))
    return World(world_template=resolve("world_template"), simulation_seed=resolve("simulation_seed"),
                 catalog=resolve("catalog"), catalog_profile=document["catalog_profile"],
                 rng_seed=document["rng_seed"], key="browser-service-proof")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--admin-url-file", default=os.environ.get("TME_PG_ADMIN_URL_FILE"),
                        required="TME_PG_ADMIN_URL_FILE" not in os.environ)
    parser.add_argument("--output", type=Path, required=True)
    arguments = parser.parse_args()
    output = arguments.output.resolve()
    if output.is_relative_to(REPOSITORY_ROOT):
        parser.error("proof output must be outside the checkout")
    if not Path("/usr/sbin/nginx").is_file():
        parser.error("native service proof requires the development nginx front end")
    output.mkdir(parents=True, exist_ok=True)
    # An interrupted or failed rerun must never leave an earlier aggregate PASS.
    (output / "verification.json").write_text(json.dumps({"verdict": "INCOMPLETE", "reason": "run has not completed"}) + "\n")
    run(["npm", "--prefix", "web", "run", "build:play"])
    roster = json.loads((REPOSITORY_ROOT / "web/proof/engines.json").read_text())
    # Each browser/scenario gets a fresh database and freshly bootstrapped actor.
    reports = []
    for scenario in ("gold_bank_locker_storage", "gold_training", "town_adventure_loop_gallery"):
        for engine in roster:
            with BrowserServer(read_admin_url(arguments.admin_url_file), fixture_world(scenario)) as server:
                config = {"origin": server.origin, "authority": str(server.authority), "engine": engine,
                          "username": server.username, "password": server.password,
                          "scenario": scenario, "output": str(output)}
                result = subprocess.run(["node", "web/proof/services-proof.mjs"], cwd=REPOSITORY_ROOT,
                                        input=json.dumps(config), text=True, capture_output=True)
                # Config and transport secrets never enter a receipt or command line.
                if result.returncode:
                    raise RuntimeError(f"{engine}/{scenario}: {result.stderr[-4000:]}")
                report = json.loads((output / f"{engine}-{scenario}.json").read_text())
                reports.append(report)
                print(f"PASS {engine}/{scenario}", flush=True)
    (output / "verification.json").write_text(json.dumps({"verdict": "PASS", "reports": reports}, indent=2) + "\n")


if __name__ == "__main__":
    main()
