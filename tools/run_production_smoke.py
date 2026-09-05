#!/usr/bin/env python3
"""Exercise the disposable production proof through public HTTPS/WebSocket."""

from __future__ import annotations

import argparse
import base64
import hashlib
import http.client
import json
import os
import secrets
import socket
import ssl
import stat
import struct
import sys
import time
import uuid
from pathlib import Path
from typing import Any, BinaryIO


PROTOCOL_MINOR = 8
CONTROL_API_VERSION = 4
WEBSOCKET_GUID = "258EAFA5-E914-47DA-95CA-C5AB0DC85B11"
WEBSOCKET_SUBPROTOCOL = "tme.v1"
CSRF_HEADER = "X-Tme-Csrf"
MAX_FRAME_BYTES = 1024 * 1024


class SmokeError(RuntimeError):
    """Raised when the production proof does not match its locked contract."""


class RoutedHTTPSConnection(http.client.HTTPSConnection):
    """Connect to one peer while retaining the public TLS authority."""

    def __init__(
        self,
        authority_host: str,
        connect_host: str,
        port: int,
        timeout: float,
        context: ssl.SSLContext,
    ) -> None:
        super().__init__(authority_host, port, timeout=timeout, context=context)
        self.connect_host = connect_host

    def connect(self) -> None:
        self.sock = socket.create_connection(
            (self.connect_host, self.port), self.timeout, self.source_address
        )
        self.sock = self._context.wrap_socket(
            self.sock, server_hostname=self.host
        )


def _read_exact(stream: BinaryIO, size: int) -> bytes:
    value = stream.read(size)
    if value is None or len(value) != size:
        raise SmokeError("WebSocket frame ended unexpectedly")
    return value


def encode_client_frame(
    opcode: int, payload: bytes, *, mask: bytes | None = None
) -> bytes:
    """Encode one final, masked client frame."""
    if len(payload) > MAX_FRAME_BYTES:
        raise SmokeError("WebSocket client frame exceeds the smoke limit")
    mask = secrets.token_bytes(4) if mask is None else mask
    if len(mask) != 4:
        raise SmokeError("WebSocket mask must contain four bytes")
    if len(payload) < 126:
        header = bytes((0x80 | opcode, 0x80 | len(payload)))
    elif len(payload) <= 0xFFFF:
        header = bytes((0x80 | opcode, 0x80 | 126)) + struct.pack("!H", len(payload))
    else:
        header = bytes((0x80 | opcode, 0x80 | 127)) + struct.pack("!Q", len(payload))
    masked = bytes(value ^ mask[index % 4] for index, value in enumerate(payload))
    return header + mask + masked


def read_server_frame(stream: BinaryIO) -> tuple[int, bytes]:
    """Read one final, unmasked server frame."""
    first, second = _read_exact(stream, 2)
    if first & 0x80 == 0:
        raise SmokeError("fragmented WebSocket server frame is unsupported")
    if first & 0x70:
        raise SmokeError("unexpected WebSocket extension bits")
    if second & 0x80:
        raise SmokeError("server sent a masked WebSocket frame")
    opcode = first & 0x0F
    size = second & 0x7F
    if size == 126:
        size = struct.unpack("!H", _read_exact(stream, 2))[0]
    elif size == 127:
        size = struct.unpack("!Q", _read_exact(stream, 8))[0]
    if size > MAX_FRAME_BYTES:
        raise SmokeError("WebSocket server frame exceeds the smoke limit")
    return opcode, _read_exact(stream, size)


class PublicClient:
    def __init__(
        self, host: str, port: int, timeout: float, connect_host: str | None = None
    ) -> None:
        self.host = host
        self.connect_host = connect_host or host
        self.port = port
        self.timeout = timeout
        authority = host if port == 443 else f"{host}:{port}"
        self.origin = f"https://{authority}"
        self.context = ssl.create_default_context()

    def request(
        self,
        method: str,
        path: str,
        *,
        body: dict[str, Any] | None = None,
        token: str | None = None,
        csrf: str | None = None,
        expected: tuple[int, ...] = (200,),
    ) -> tuple[dict[str, Any] | None, list[tuple[str, str]]]:
        payload = None if body is None else json.dumps(body, separators=(",", ":"))
        headers = {"Accept": "application/json", "Origin": self.origin}
        if payload is not None:
            headers["Content-Type"] = "application/json"
        if token is not None:
            headers["Authorization"] = "Bearer " + token
        if csrf is not None:
            headers[CSRF_HEADER] = csrf
        connection = RoutedHTTPSConnection(
            self.host,
            self.connect_host,
            self.port,
            self.timeout,
            self.context,
        )
        try:
            connection.request(method, path, body=payload, headers=headers)
            response = connection.getresponse()
            raw_body = response.read()
            response_headers = response.getheaders()
        except (OSError, http.client.HTTPException, ssl.SSLError) as error:
            raise SmokeError(f"{method} {path} failed: {error}") from error
        finally:
            connection.close()
        if response.status not in expected:
            raise SmokeError(
                f"{method} {path} returned unexpected HTTP {response.status}"
            )
        if not raw_body:
            return None, response_headers
        try:
            value = json.loads(raw_body)
        except json.JSONDecodeError as error:
            raise SmokeError(f"{method} {path} returned invalid JSON") from error
        if not isinstance(value, dict):
            raise SmokeError(f"{method} {path} did not return a JSON object")
        return value, response_headers

    def login(self, username: str, password: str) -> "AuthenticatedClient":
        response, headers = self.request(
            "POST", "/v4/login", body={"username": username, "password": password}
        )
        if response is None or set(response) != {"session_token", "bootstrap"}:
            raise SmokeError("login response shape was refused")
        if any(name.lower() == "set-cookie" for name, _ in headers):
            raise SmokeError("login emitted a retired cookie")
        bootstrap = response["bootstrap"]
        _validate_session_bootstrap(bootstrap)
        token = _required_string(response, "session_token")
        csrf = bootstrap.get("csrf_token")
        if not isinstance(csrf, str) or not csrf:
            raise SmokeError("login response omitted the CSRF token")
        return AuthenticatedClient(self, token, csrf, bootstrap)


class AuthenticatedClient:
    def __init__(
        self,
        public: PublicClient,
        token: str,
        csrf: str,
        bootstrap: dict[str, Any],
    ) -> None:
        self.public = public
        self.token = token
        self.csrf = csrf
        self.bootstrap = bootstrap

    def session(self) -> dict[str, Any]:
        value, _ = self.public.request("POST", "/v4/session", body={}, token=self.token)
        if value is None:
            raise SmokeError("session response was empty")
        _validate_session_bootstrap(value)
        csrf = value.get("csrf_token")
        if not isinstance(csrf, str) or not csrf:
            raise SmokeError("session response omitted the refreshed CSRF token")
        self.csrf = csrf
        self.bootstrap = value
        return value

    def character(self, slot: int) -> dict[str, Any]:
        characters = self.bootstrap.get("characters")
        if not isinstance(characters, list):
            raise SmokeError("session bootstrap omitted characters")
        matches = [row for row in characters if isinstance(row, dict) and row.get("slot") == slot]
        if len(matches) != 1:
            raise SmokeError(f"session bootstrap does not contain exactly one slot {slot}")
        return matches[0]

    def select(self, slot: int) -> dict[str, Any]:
        character = self.character(slot)
        character_id = character.get("character_id")
        value, _ = self.public.request(
            "POST",
            "/v4/characters/select",
            token=self.token,
            body={"csrf_token": self.csrf, "character_id": character_id},
        )
        if value is None:
            raise SmokeError("character selection response was empty")
        if value.get("control_api_version") != CONTROL_API_VERSION:
            raise SmokeError("character selection response used the wrong control API version")
        self.bootstrap["selected_character_id"] = character_id
        return character

    def ticket(self) -> str:
        value, _ = self.public.request(
            "POST",
            "/v4/socket-tickets",
            token=self.token,
            body={"csrf_token": self.csrf},
        )
        ticket = None if value is None else value.get("ticket")
        if not isinstance(ticket, str) or not ticket:
            raise SmokeError("socket ticket response omitted the ticket")
        if value.get("protocol_major") != 1 or value.get("supported_minors") != [
            PROTOCOL_MINOR
        ]:
            raise SmokeError("socket ticket response did not require protocol 1.8")
        return ticket

    def connect(self) -> "GameplaySocket":
        return GameplaySocket(self.public, self.ticket())

    def logout(self) -> None:
        self.public.request(
            "POST",
            "/v4/logout",
            token=self.token,
            csrf=self.csrf,
            body={"csrf_token": self.csrf},
            expected=(204,),
        )

    def forgive(self, mark_id: str) -> dict[str, Any]:
        value, _ = self.public.request(
            "POST",
            f"/v4/player-kill-marks/{mark_id}/forgive",
            token=self.token,
            csrf=self.csrf,
            body={"request_id": str(uuid.uuid4())},
        )
        if value is None:
            raise SmokeError("forgiveness response was empty")
        if value.get("control_api_version") != CONTROL_API_VERSION:
            raise SmokeError("forgiveness response used the wrong control API version")
        return value


class GameplaySocket:
    def __init__(self, public: PublicClient, ticket: str) -> None:
        self.public = public
        self.latest_state: dict[str, Any] = {}
        self.state_generation = 0
        self.raw = socket.create_connection(
            (public.connect_host, public.port), timeout=public.timeout
        )
        self.socket = public.context.wrap_socket(self.raw, server_hostname=public.host)
        self.socket.settimeout(public.timeout)
        self.stream = self.socket.makefile("rb")
        key = base64.b64encode(secrets.token_bytes(16)).decode("ascii")
        authority = public.host if public.port == 443 else f"{public.host}:{public.port}"
        request = (
            "GET /v4/socket HTTP/1.1\r\n"
            f"Host: {authority}\r\n"
            f"Origin: {public.origin}\r\n"
            "Upgrade: websocket\r\n"
            "Connection: Upgrade\r\n"
            f"Sec-WebSocket-Key: {key}\r\n"
            "Sec-WebSocket-Version: 13\r\n"
            f"Sec-WebSocket-Protocol: {WEBSOCKET_SUBPROTOCOL}\r\n\r\n"
        )
        self.socket.sendall(request.encode("ascii"))
        status = _read_exact_line(self.stream)
        headers = _read_headers(self.stream)
        if not status.startswith("HTTP/1.1 101 "):
            raise SmokeError(f"WebSocket upgrade failed: {status}")
        expected_accept = base64.b64encode(
            hashlib.sha1((key + WEBSOCKET_GUID).encode("ascii")).digest()
        ).decode("ascii")
        if headers.get("sec-websocket-accept") != expected_accept:
            raise SmokeError("WebSocket upgrade returned the wrong accept value")
        if headers.get("sec-websocket-protocol") != WEBSOCKET_SUBPROTOCOL:
            raise SmokeError("WebSocket upgrade returned the wrong subprotocol")
        self.send_json(
            {
                "kind": "client_hello",
                "ticket": ticket,
                "supported_minors": [PROTOCOL_MINOR],
            }
        )
        welcome = self.receive_json()
        if welcome.get("kind") != "server_welcome":
            raise SmokeError(f"expected server_welcome, received {welcome.get('kind')!r}")
        if welcome.get("selected_minor") != PROTOCOL_MINOR:
            raise SmokeError("server selected an unexpected protocol minor")
        self.welcome = welcome
        self.actor_id = _required_string(welcome, "actor_id")
        self.control_epoch = _required_decimal(welcome, "control_epoch")
        self.world_revision = _required_decimal(welcome, "world_revision")
        self.client_sequence = 0

    def send_json(self, value: dict[str, Any]) -> None:
        payload = json.dumps(value, separators=(",", ":")).encode("utf-8")
        self.socket.sendall(encode_client_frame(0x1, payload))

    def receive_json(self) -> dict[str, Any]:
        while True:
            opcode, payload = read_server_frame(self.stream)
            if opcode == 0x9:
                self.socket.sendall(encode_client_frame(0xA, payload))
                continue
            if opcode == 0x8:
                raise SmokeError("server closed the WebSocket during the smoke")
            if opcode != 0x1:
                raise SmokeError(f"unexpected WebSocket opcode {opcode}")
            try:
                value = json.loads(payload)
            except (UnicodeDecodeError, json.JSONDecodeError) as error:
                raise SmokeError("server sent invalid WebSocket JSON") from error
            if not isinstance(value, dict):
                raise SmokeError("server WebSocket payload was not an object")
            if value.get("kind") == "state_update":
                self.world_revision = _required_decimal(value, "world_revision")
            if value.get("kind") in ("state_update", "server_welcome"):
                self.latest_state = value
                self.state_generation += 1
            return value

    def command(
        self,
        intent: dict[str, Any],
        *,
        envelope: dict[str, Any] | None = None,
    ) -> tuple[dict[str, Any], dict[str, Any]]:
        if envelope is None:
            self.client_sequence += 1
            envelope = {
                "kind": "command",
                "command_id": str(uuid.uuid4()),
                "control_epoch": str(self.control_epoch),
                "client_sequence": str(self.client_sequence),
                "observed_world_revision": str(self.world_revision),
                "actor_id": self.actor_id,
                "intent": intent,
            }
        command_id = _required_string(envelope, "command_id")
        self.send_json(envelope)
        while True:
            value = self.receive_json()
            if value.get("kind") == "command_result" and value.get("command_id") == command_id:
                after_revision = value.get("after_revision")
                if after_revision is not None:
                    self.world_revision = int(after_revision)
                return value, envelope

    def close(self) -> None:
        try:
            self.socket.sendall(encode_client_frame(0x8, struct.pack("!H", 1000)))
        except OSError:
            pass
        self.stream.close()
        self.socket.close()


def _read_exact_line(stream: BinaryIO) -> str:
    raw = stream.readline(8193)
    if not raw.endswith(b"\r\n") or len(raw) > 8192:
        raise SmokeError("invalid WebSocket HTTP response line")
    return raw[:-2].decode("ascii", errors="strict")


def _read_headers(stream: BinaryIO) -> dict[str, str]:
    headers: dict[str, str] = {}
    while True:
        line = _read_exact_line(stream)
        if not line:
            return headers
        name, separator, value = line.partition(":")
        if not separator:
            raise SmokeError("invalid WebSocket HTTP response header")
        headers[name.lower()] = value.strip()


def _required_string(value: dict[str, Any], key: str) -> str:
    field = value.get(key)
    if not isinstance(field, str) or not field:
        raise SmokeError(f"wire response omitted {key}")
    return field


def _required_decimal(value: dict[str, Any], key: str) -> int:
    field = _required_string(value, key)
    try:
        return int(field)
    except ValueError as error:
        raise SmokeError(f"wire response has invalid {key}") from error


def _validate_session_bootstrap(bootstrap: dict[str, Any]) -> None:
    if bootstrap.get("control_api_version") != CONTROL_API_VERSION:
        raise SmokeError("session bootstrap used the wrong control API version")
    for retired in ("facets", "facet_id", "switch_facet"):
        if retired in bootstrap:
            raise SmokeError(f"session bootstrap exposed the retired field {retired}")
    characters = bootstrap.get("characters")
    if not isinstance(characters, list):
        raise SmokeError("session bootstrap omitted characters")
    slots: set[Any] = set()
    for entry in characters:
        if not isinstance(entry, dict):
            raise SmokeError("character summary was not an object")
        slot = entry.get("slot")
        if not isinstance(slot, int) or slot in slots:
            raise SmokeError("character summary carried an invalid or duplicate slot")
        slots.add(slot)
        if not isinstance(entry.get("character_id"), str) or not entry["character_id"]:
            raise SmokeError("character summary omitted its character ID")


def read_protected_credential(path: str) -> str:
    credential = Path(path)
    try:
        metadata = credential.lstat()
    except OSError as error:
        raise SmokeError("protected smoke credential is unavailable") from error
    mode = stat.S_IMODE(metadata.st_mode)
    readable_projection = metadata.st_uid == os.geteuid() and mode == 0o400
    readable_root_source = metadata.st_uid == 0 and mode == 0o400 and os.geteuid() == 0
    try:
        parent = credential.parent.lstat()
        canonical_parent = credential.parent.resolve(strict=True)
    except OSError as error:
        raise SmokeError("protected smoke credential parent is unavailable") from error
    parent_safe = (
        stat.S_ISDIR(parent.st_mode)
        and (
            (parent.st_uid == os.geteuid() and stat.S_IMODE(parent.st_mode) == 0o700)
            or (parent.st_uid == 0 and stat.S_IMODE(parent.st_mode) == 0o700 and os.geteuid() == 0)
        )
        and canonical_parent == credential.parent
    )
    if (
        not stat.S_ISREG(metadata.st_mode)
        or not (readable_projection or readable_root_source)
        or not parent_safe
        or metadata.st_nlink != 1
        or metadata.st_size < 1
        or metadata.st_size > 8 * 1024
    ):
        raise SmokeError("protected smoke credential metadata is invalid")
    try:
        raw = credential.read_bytes()
    except (OSError, UnicodeError) as error:
        raise SmokeError("protected smoke credential is unreadable") from error
    if not raw.endswith(b"\n") or raw.count(b"\n") != 1 or any(
        character in raw for character in (b"\r", b"\0")
    ):
        raise SmokeError("protected smoke credential content is invalid")
    try:
        value = raw[:-1].decode("utf-8")
    except UnicodeError as error:
        raise SmokeError("protected smoke credential content is invalid") from error
    if not value or any(
        token in value.upper() for token in ("REQUIRED", "PLACEHOLDER", "CHANGEME", "TODO")
    ):
        raise SmokeError("protected smoke credential content is invalid")
    return value


def _mark_state(session: dict[str, Any]) -> dict[str, Any]:
    state = session.get("player_kill_marks")
    if not isinstance(state, dict):
        raise SmokeError("session bootstrap omitted player-kill mark state")
    return state


def _admission_preference_intent(gameplay: "GameplaySocket") -> dict[str, Any]:
    frame = gameplay.welcome.get("frame")
    social = frame.get("social") if isinstance(frame, dict) else None
    pages_enabled = social.get("pages_enabled") if isinstance(social, dict) else None
    if not isinstance(pages_enabled, bool):
        raise SmokeError("observer frame omitted page preference")
    return {"kind": "set_pages_enabled", "enabled": not pages_enabled}


def _enabled_unsafe_attack(gameplay: GameplaySocket, target_actor_id: str) -> dict[str, Any]:
    frame = gameplay.welcome.get("frame")
    if not isinstance(frame, dict):
        raise SmokeError("server welcome omitted the observer frame")
    options = frame.get("action_options")
    if not isinstance(options, list):
        raise SmokeError("server welcome omitted action options")
    candidates = []
    for option in options:
        if not isinstance(option, dict) or option.get("enabled") is not True:
            continue
        intent = option.get("intent")
        if (
            isinstance(intent, dict)
            and intent.get("kind") == "physical_attack"
            and intent.get("target_actor_id") == target_actor_id
            and intent.get("authorization") == "confirmed_unsafe"
        ):
            candidates.append(intent)
    if not candidates:
        raise SmokeError("observer frame exposed no enabled unsafe physical attack")
    return dict(candidates[0])


def run_smoke(
    host: str,
    connect_host: str,
    port: int,
    username_one: str,
    password_one: str,
    username_two: str,
    password_two: str,
    reload_wait_seconds: float,
    profile: str,
) -> None:
    public = PublicClient(host, port, timeout=15.0, connect_host=connect_host)
    ready, _ = public.request("GET", "/health/ready")
    if ready is None:
        raise SmokeError("readiness response was empty")
    public.request("GET", "/internal/status", expected=(404,))
    print("production-smoke: trusted public TLS and route boundary passed", flush=True)

    first = public.login(username_one, password_one)
    second = public.login(username_two, password_two)
    first.select(1)
    second.select(1)
    first_socket = first.connect()
    second_socket = second.connect()
    if first_socket.actor_id == second_socket.actor_id:
        raise SmokeError("both proof accounts bound the same actor")
    if reload_wait_seconds > 0:
        print("production-smoke: websocket ready for Caddy reload", flush=True)
        time.sleep(reload_wait_seconds)

    if profile == "admission":
        preference_intent = _admission_preference_intent(first_socket)
        original_pages_enabled = not preference_intent["enabled"]
        changed, replay_envelope = first_socket.command(preference_intent)
        if changed.get("disposition") != {"kind": "accepted"}:
            raise SmokeError("repeatable admission preference command was not accepted")
        replayed, _ = first_socket.command(
            replay_envelope["intent"], envelope=replay_envelope
        )
        if replayed.get("replay_status") != "replayed":
            raise SmokeError("admission command retry was not a durable replay")
        restored, _ = first_socket.command(
            {"kind": "set_pages_enabled", "enabled": original_pages_enabled}
        )
        if restored.get("disposition") != {"kind": "accepted"}:
            raise SmokeError("admission preference was not restored")
        print(
            "production-smoke: authenticated preference toggle, replay, and restore passed",
            flush=True,
        )
    else:
        unsafe_intent = _enabled_unsafe_attack(first_socket, second_socket.actor_id)
        safe_intent = dict(unsafe_intent)
        safe_intent["authorization"] = "safe"
        safe, _ = first_socket.command(safe_intent)
        if safe.get("disposition") != {"kind": "rejected", "code": "rules_rejected"}:
            raise SmokeError("safe PvP command was not rules-rejected")
        unsafe, _ = first_socket.command(unsafe_intent)
        if unsafe.get("disposition") != {"kind": "accepted"}:
            raise SmokeError("confirmed unsafe PvP command was not accepted")

        deadline = time.monotonic() + 5.0
        victim_marks: list[Any] = []
        while time.monotonic() < deadline:
            victim_state = _mark_state(second.session())
            candidate = victim_state.get("forgivable_marks")
            if isinstance(candidate, list) and candidate:
                victim_marks = candidate
                break
            time.sleep(0.25)
        if len(victim_marks) != 1 or not isinstance(victim_marks[0], dict):
            raise SmokeError("unsafe PvP did not create exactly one victim-forgivable mark")
        killer_state = _mark_state(first.session())
        if killer_state.get("active_count") != 1:
            raise SmokeError("killer account did not expose exactly one active mark")
        mark_id = _required_string(victim_marks[0], "mark_id")
        forgiven = second.forgive(mark_id)
        if forgiven.get("mark_id") != mark_id or forgiven.get("replay_status") != "new":
            raise SmokeError("victim forgiveness did not commit as a new result")
        if _mark_state(first.session()).get("active_count") != 0:
            raise SmokeError("forgiven mark remained active on the killer account")
        print("production-smoke: safe/unsafe PvP, mark, and victim forgiveness passed", flush=True)

    first_socket.close()
    second_socket.close()
    reconnect = first.connect()
    if reconnect.actor_id != first_socket.actor_id:
        raise SmokeError("reconnect selected the wrong actor")
    reconnect.close()

    first.session()
    second.session()
    first.logout()
    second.logout()
    print("production-smoke: reconnect and two-account session teardown passed", flush=True)


def parse_args(argv: list[str]) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--host", required=True)
    parser.add_argument("--connect-host")
    parser.add_argument("--port", type=int, default=443)
    parser.add_argument("--username-one-file", required=True)
    parser.add_argument("--password-one-file", required=True)
    parser.add_argument("--username-two-file", required=True)
    parser.add_argument("--password-two-file", required=True)
    parser.add_argument("--profile", choices=("admission", "pvp"), default="admission")
    parser.add_argument("--reload-wait-seconds", type=float, default=0.0)
    return parser.parse_args(argv)


def main(argv: list[str]) -> int:
    args = parse_args(argv)
    try:
        username_one = read_protected_credential(args.username_one_file)
        password_one = read_protected_credential(args.password_one_file)
        username_two = read_protected_credential(args.username_two_file)
        password_two = read_protected_credential(args.password_two_file)
        run_smoke(
            args.host,
            args.connect_host or args.host,
            args.port,
            username_one,
            password_one,
            username_two,
            password_two,
            args.reload_wait_seconds,
            args.profile,
        )
    except SmokeError as error:
        print(f"production-smoke: FAIL: {error}", file=sys.stderr)
        return 1
    print("production-smoke: PASS", flush=True)
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
