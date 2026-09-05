#!/usr/bin/env python3
"""Prove the restored Workbench operation against live and replayed browser frames."""
from __future__ import annotations

import argparse
import json
import os
import shutil
import subprocess
import tempfile
import threading
import urllib.request
from http.server import ThreadingHTTPServer
from pathlib import Path

from workbench import serve
from workbench.identity import resolve
from workbench.resolve import resolve_packet
from workbench.projection import StaleSelection

ROOT = Path(__file__).resolve().parents[1]
PROJECTION = "content/lands/identity-proof/generated/workbench_projection.json"
WORLD = "content/lands/identity-proof/world.json"


def proof(admin_url_file: str, output: Path) -> None:
    workbench = serve.Workbench(ROOT, PROJECTION, None, {"world_document": WORLD, "admin_url_file": admin_url_file})
    handler = type("CaptureProofHandler", (serve.Handler,), {"workbench": workbench})
    server = ThreadingHTTPServer(("127.0.0.1", 0), handler)
    thread = threading.Thread(target=server.serve_forever, daemon=True)
    thread.start()
    base = f"http://127.0.0.1:{server.server_port}"

    def post(route, body):
        request = urllib.request.Request(base + route, data=json.dumps(body).encode(), headers={"Content-Type": "application/json"})
        with urllib.request.urlopen(request, timeout=600) as response:
            return json.load(response)

    try:
        output.mkdir(parents=True, exist_ok=True)
        ui_report = output / "workbench-ui.json"
        result = subprocess.run(["node", "web/proof/workbench-capture.mjs"], cwd=ROOT,
            input=json.dumps({"base": base, "output": str(ui_report)}), text=True, timeout=900)
        if result.returncode:
            raise AssertionError("Workbench browser UI proof failed")
        offered = json.loads(ui_report.read_bytes())["captures"]
        assert len(offered) == 4, "live and replay must run in both engines"
        for row in offered:
            taken = workbench.capture(row["capture_id"])
            frame = json.loads(json.loads((taken.directory / "capture.frame.json").read_bytes())["envelopes"][-1])["frame"]
            expected = {("tile", f"{r['position']['x']}:{r['position']['y']}"): r["position"] for r in frame["tiles"]}
            for kind, collection, identity, location in (
                ("actor", "actors", "actor_id", "position"), ("corpse", "corpses", "corpse_id", "location"),
                ("ground_item", "ground_items", "item_instance_id", "location"), ("gold_pile", "gold_piles", "gold_pile_id", "location"),
            ):
                expected.update({(kind, r[identity]): r[location]["position"] for r in frame[collection]})
            actual = {(r["kind"], r["source_identity"]): r["coordinate"] for r in taken.targets}
            assert actual == expected, "browser target identities or coordinates differ from the observed frame"
            # All cells are in the compiler's lattice; point through real HTTP
            # for both a square and every kind of occupant this frame contains.
            member = workbench.projection.member(taken.level)
            for target in taken.targets:
                coord = target["coordinate"]
                assert member.contains((coord["x"], coord["y"]))
            chosen = {target["kind"]: target for target in taken.targets}
            for target in chosen.values():
                result = post("/api/capture/selection", {"capture_id": row["capture_id"], "gesture": "click", "point": target["anchor"]})
                packet = result["packet"]
                coord = target["coordinate"]
                logical = resolve(member, [(coord["x"], coord["y"])])
                assert packet["semantic"] == logical["semantic"]
                packet_path = workbench.session.directory / "selections" / (packet["selection_id"] + ".json")
                resolve_packet(packet_path, ROOT)
            # A changed recording must kill both an existing packet and the
            # still-cached capture before another selection can be written.
            recorded_path = taken.directory / "capture.frame.json"
            saved = recorded_path.read_bytes()
            try:
                recorded_path.write_bytes(saved + b"\n")
                for operation in (lambda: resolve_packet(packet_path, ROOT), lambda: workbench.capture(row["capture_id"])):
                    try:
                        operation()
                    except StaleSelection:
                        pass
                    else:
                        raise AssertionError("changed authoritative recording was accepted")
            finally:
                recorded_path.write_bytes(saved)
        output.mkdir(parents=True, exist_ok=True)
        shutil.copytree(workbench.session.directory / "captures", output / "captures", dirs_exist_ok=True)
        (output / "proof.json").write_text(json.dumps({"captures": 4, "http_operation": True, "logical_correspondence": True,
            "recording_mutant_killed": True, "cached_capture_mutant_killed": True}, indent=2) + "\n")
        print(f"browser capture proof: {output}")
        print("TME_BROWSER_CAPTURE_PROOF_OK")
    finally:
        server.shutdown(); server.server_close(); thread.join(timeout=5)
        shutil.rmtree(workbench.session.directory)


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--admin-url-file", default=os.environ.get("TME_PG_ADMIN_URL_FILE"), required="TME_PG_ADMIN_URL_FILE" not in os.environ)
    parser.add_argument("--output", type=Path)
    arguments = parser.parse_args()
    if arguments.output:
        proof(arguments.admin_url_file, arguments.output)
    else:
        with tempfile.TemporaryDirectory(prefix="tme-browser-capture-proof-") as directory:
            proof(arguments.admin_url_file, Path(directory))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
