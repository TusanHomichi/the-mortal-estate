"""Two real clients against the installed private authority, including restart."""
from __future__ import annotations

import json
import sys
import time
from contextlib import ExitStack
from types import SimpleNamespace

from common import REPO, UNITS, document
from operations import backup, health, restore_drill

sys.path.insert(0, str(REPO / "tools"))
from live_wire_client import LiveWireClient


def prove(site, *, restart=False):
    health(site)
    accounts = json.loads((site.config / "test-accounts.json").read_text())
    characters = json.loads((site.config / "bootstrap.json").read_text())["characters"]
    observations = []
    with ExitStack() as cleanup:
        clients = []
        for account in accounts:
            character = next(row for row in characters if row["account_id"] == account["account_id"])
            server = SimpleNamespace(origin=site.origin, authority=site.config / "tls/ca.pem",
                                     username=account["username"], password=account["password"], character_id=character["character_id"])
            clients.append(cleanup.enter_context(LiveWireClient(server)))
        first, second = clients
        ids = {client.gameplay.actor_id for client in clients}
        for client in clients:
            client.wait_for(lambda frame: ids <= {row["actor_id"] for row in frame["actors"]} and frame["can_act"])

        def start(client):
            result, _ = client.command({"kind": "wait"})
            assert result["disposition"] == {"kind": "accepted"}
            frame = client.wait_for(lambda frame: not frame["can_act"])
            started, ready = int(frame["logical_time"]), int(frame["ready_at"])
            assert ready - started == 3000, "action did not receive a complete individual interval"
            return started, ready

        start_one, ready_one = start(first)
        time.sleep(.8)
        start_two, ready_two = start(second)
        assert ready_two > ready_one and start_two > start_one
        rejected, _ = second.command({"kind": "wait"})
        assert rejected["disposition"]["kind"] == "rejected"
        assert int(second.frame["ready_at"]) == ready_two
        first.gameplay.close()
        first.gameplay = first.session.connect()
        assert not first.frame["can_act"] and int(first.frame["ready_at"]) == ready_one
        ready = first.wait_for(lambda frame: frame["can_act"])
        second.wait_for(lambda frame: int(frame["logical_time"]) >= int(ready["logical_time"]))
        assert not second.frame["can_act"], "independent actions became ready together"
        second.wait_for(lambda frame: frame["can_act"])
        observations.append({"first_start": start_one, "first_ready": ready_one,
                             "second_start": start_two, "second_ready": ready_two,
                             "reconnect_preserved_deadline": True, "mutual_visibility": True})
        if restart:
            _, deadline = start(first)
            site.service("stop", *reversed(UNITS))
            for client in clients:
                client.gameplay.close()
            site.service("start", *UNITS)
            health(site)
            for client in clients:
                client.session.session()
                client.gameplay = client.session.connect()
            assert not first.frame["can_act"] and int(first.frame["ready_at"]) == deadline, "restart cleared the remaining action interval"
            first.wait_for(lambda frame: frame["can_act"])
            observations.append({"restart_preserved_deadline": True, "deadline": deadline})
        tokens = [(client.public, client.session.token) for client in clients]
    for public, token in tokens:
        public.request("POST", "/v4/session", body={}, token=token, expected=(401,))
    saved = backup(site)
    restored = restore_drill(site, saved)
    result = {"observations": observations, "old_sessions_refused": True,
              "backup": str(saved), "restore": restored, "status": health(site)}
    document(site.config / "deployment-proof.json", result)
    return result
