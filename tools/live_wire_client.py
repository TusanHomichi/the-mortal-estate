"""Observe a scratch server through the existing HTTPS/WebSocket transport.

This is protocol evidence, not a rendered browser or a second gameplay client.
The production smoke owns transport and authentication; the Rust protocol owns
the schema. Every frame here comes from the real server.
"""

from __future__ import annotations

import time
from urllib.parse import urlsplit

from live_server_harness import LiveServer, ProofError
from run_production_smoke import PublicClient


class LiveWireClient:
    def __init__(self, server: LiveServer, timeout: float = 30.0):
        endpoint = urlsplit(server.origin)
        self.public = PublicClient(endpoint.hostname, endpoint.port, timeout)
        self.public.context.load_verify_locations(cafile=str(server.authority))
        self.server = server
        self.session = None
        self.gameplay = None

    def __enter__(self):
        try:
            self.session = self.public.login(self.server.username, self.server.password)
            characters = self.session.bootstrap["characters"]
            selected = next(row for row in characters if row["character_id"] == self.server.character_id)
            self.session.select(selected["slot"])
            self.gameplay = self.session.connect()
            self.frame
            return self
        except BaseException:
            self.__exit__(None, None, None)
            raise

    def __exit__(self, *_):
        try:
            if self.session is not None:
                self.session.logout()
        finally:
            if self.gameplay is not None:
                self.gameplay.close()

    @property
    def frame(self):
        frame = self.gameplay.latest_state.get("frame")
        if not isinstance(frame, dict) or frame.get("contract_version") != 8:
            raise ProofError("live wire proof requires observer contract 8")
        return frame

    def wait_for(self, predicate, timeout: float = 30.0):
        until = time.monotonic() + timeout
        previous_timeout = self.gameplay.socket.gettimeout()
        try:
            while not predicate(self.frame):
                remaining = until - time.monotonic()
                if remaining <= 0:
                    raise ProofError("no authoritative frame satisfied the observation deadline")
                self.gameplay.socket.settimeout(remaining)
                self.gameplay.receive_json()
            return self.frame
        finally:
            self.gameplay.socket.settimeout(previous_timeout)

    def command(self, intent):
        return self.gameplay.command(intent)
