#!/usr/bin/env python3
"""Native first-expedition UI proof against its authored world and scratch PostgreSQL."""
from __future__ import annotations
import argparse
import json
from pathlib import Path
import subprocess
from live_server_harness import REPOSITORY_ROOT, World, read_admin_url, run
from run_browser_services_proof import BrowserFront, BrowserServer
from run_death_return_proof import (
    LedgerAudit,
    ProofHandshake,
    prove_restart_durability,
    run_proof_child,
)


class ExpeditionServer(BrowserServer):
    presentation_assets: Path

    def start_proxy(self, listen_port, upstream_port, certificate, key):
        return BrowserFront(self.run_directory, REPOSITORY_ROOT / "web/dist/play", listen_port,
                            upstream_port, certificate, key, self.presentation_assets)


def creation_alignments() -> dict[str, str]:
    """The authored alignment every creation profile selects, read from content."""
    catalog = json.loads(
        (REPOSITORY_ROOT / "content/lands/first-expedition/catalog.json").read_text()
    )
    return {
        profile_id: profile["character"]["alignment_state"]["alignment"]
        for profile_id, profile in catalog["creation_profiles"].items()
    }


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--admin-url-file", required=True, type=Path)
    parser.add_argument("--assets", required=True, type=Path)
    parser.add_argument("--output", required=True, type=Path)
    parser.add_argument("--engine", choices=["chromium", "firefox", "webkit"])
    parser.add_argument("--journey", choices=["success", "death"], default="success",
                        help="which composed journey to run on the unmodified release world")
    parser.add_argument("--alignment", choices=["lawful", "neutral"],
                        help="which authored return destination to run, for the death journey")
    arguments = parser.parse_args()
    output = arguments.output.resolve()
    if output.is_relative_to(REPOSITORY_ROOT):
        parser.error("proof output must be outside the checkout")
    output.mkdir(parents=True, exist_ok=True)
    receipt = output / "verification.json"
    receipt.write_text(json.dumps({"verdict": "INCOMPLETE"}) + "\n")
    run(["npm", "--prefix", "web", "run", "build:play"])
    roster = json.loads((REPOSITORY_ROOT / "web/proof/engines.json").read_text())
    # The return destination is the served world's own authored policy, and which
    # of the two branches applies is the character's alignment. Every authored
    # creation profile selects `lawful`, so a character created through the
    # ordinary UI can only reach the lawful destination; the neutral branch is
    # proved by `crates/tme-rules/tests/first_expedition_return_destination.rs`,
    # which reaches it through the ordinary promotion rule instead of by
    # assigning an alignment the creation flow cannot produce.
    destinations = json.loads(
        (REPOSITORY_ROOT / "content/lands/first-expedition/generated/world_template.json").read_text()
    )["resurrection"]["first_expedition"]
    policy = {
        "lawful_destination": destinations["lawful_destination"],
        "neutral_destination": destinations["neutral_destination"],
        "request_delay_ms": destinations["request_delay_ms"],
        "selected_by": "the created character's authored alignment",
        "creation_profile_alignments": creation_alignments(),
    }
    cases = [("lawful", "lawful_destination")] if arguments.journey == "death" else [(None, None)]
    if arguments.alignment and arguments.alignment != "lawful":
        parser.error(
            "no authored creation profile selects the neutral alignment, so the composed death "
            "journey has no supported ordinary creation choice that reaches the neutral "
            "destination; it is proved by the rules test instead"
        )
    reports = []
    for engine in roster:
        if arguments.engine and engine != arguments.engine:
            continue
        for alignment, destination_key in cases:
            server = ExpeditionServer(read_admin_url(arguments.admin_url_file),
                                      World.declared("content/lands/first-expedition/world.json"))
            server.presentation_assets = arguments.assets
            with server:
                config = dict(origin=server.origin, authority=str(server.authority), engine=engine,
                              username=server.username, password=server.password, output=str(output),
                              journey=arguments.journey,
                              destination=destinations[destination_key] if destination_key else None)
                if alignment:
                    config["alignment"] = alignment
                # The handshake file names are the browser's own: they are built
                # from the engine and the journey, which is what the proof half
                # knows. An extra suffix here would leave the child waiting for a
                # file this runner never writes.
                stem = f"{engine}-{arguments.journey}"
                request_path = output / f"{stem}-restart-request.json"
                complete_path = output / f"{stem}-restart-complete.json"
                ledger_path = output / f"{stem}-ledger-request.json"
                ledger_complete_path = output / f"{stem}-ledger-complete.json"
                ledger = LedgerAudit(server.database_url)

                def on_restart_request(requested: dict) -> dict:
                    """Replace the serving process while the browser waits."""
                    restart = prove_restart_durability(server, expect=requested)
                    restart["origin"] = server.origin
                    return restart

                # The route is a full journey: cross-town travel, an eight-round
                # fight, training, a locker round trip, reconnects and sign-out all
                # run in real time against a real server. The default budget is not
                # a statement about the product, so it is generous and explicit.
                result = run_proof_child(
                    ["node", "web/proof/expedition-proof.mjs"],
                    configuration=config,
                    request_path=request_path,
                    complete_path=complete_path,
                    on_restart_request=(
                        on_restart_request
                        if arguments.journey == "death"
                        else None
                    ),
                    # The item and coin ledger is read from the stored checkpoint
                    # on the browser's behalf, so the conservation assertion is
                    # against the authoritative world rather than against frames.
                    extra_handshakes=[
                        ProofHandshake("ledger", ledger_path, ledger_complete_path, ledger.answer)
                    ],
                    timeout=1800,
                    cwd=REPOSITORY_ROOT,
                )
                if result.returncode:
                    raise RuntimeError(f"{engine}/{arguments.journey}: {result.stderr[-5000:]}")
                report = json.loads(
                    (output / f"{engine}-expedition.json").read_text()
                )
                report["alignment"] = alignment
                report["destination_policy"] = policy
                reports.append(report)
                print(f"PASS {engine}/{arguments.journey}{'/' + alignment if alignment else ''}",
                      flush=True)
    receipt.write_text(json.dumps(dict(verdict="PASS", destination_policy=policy,
                                     reports=reports), indent=2) + "\n")


if __name__ == "__main__":
    main()
