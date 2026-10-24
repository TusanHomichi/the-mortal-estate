"""Wire observation preserves interleaved state and bounded read deadlines."""

import json
from types import SimpleNamespace
import unittest
from unittest.mock import Mock, patch

from live_server_harness import ProofError
from live_wire_client import LiveWireClient
from run_production_smoke import GameplaySocket


class WireObservation(unittest.TestCase):
    def test_the_session_selects_the_character_the_caller_names(self):
        # One account can own both the harness's enrolled character and a
        # character a proof created through the UI. A session that always took
        # the harness's own row observed a different actor than the browser was
        # driving, which is what the composed death journey hit.
        slots = []
        gameplay = SimpleNamespace(latest_state={"frame": {"contract_version": 9}})
        session = SimpleNamespace(
            bootstrap={"characters": [
                {"character_id": "harness", "slot": 1},
                {"character_id": "created", "slot": 2},
            ]},
            select=slots.append,
            connect=Mock(return_value=gameplay),
            logout=Mock(),
        )
        server = SimpleNamespace(origin="https://localhost:1", authority="/dev/null",
                                 username="u", password="p", character_id="harness")
        for character_id, expected_slot in [(None, 1), ("created", 2)]:
            slots.clear()
            with patch("live_wire_client.PublicClient") as public:
                public.return_value.login.return_value = session
                client = LiveWireClient(server, character_id=character_id)
                client.public.context = Mock()
                client.__enter__()
                self.assertEqual(client.frame["contract_version"], 9)
            self.assertEqual(slots, [expected_slot], character_id)

    def test_command_keeps_state_received_before_its_result(self):
        gameplay = GameplaySocket.__new__(GameplaySocket)
        gameplay.latest_state = {}
        gameplay.state_generation = 0
        gameplay.stream = Mock()
        gameplay.send_json = Mock()
        state = {"kind": "state_update", "world_revision": "12", "frame": {"can_act": False}}
        result = {"kind": "command_result", "command_id": "request", "after_revision": "12"}
        frames = [(1, json.dumps(value).encode()) for value in [state, result]]
        with patch("run_production_smoke.read_server_frame", side_effect=frames):
            observed, _ = gameplay.command({}, envelope={"command_id": "request"})
        self.assertEqual(observed, result)
        self.assertEqual(gameplay.latest_state, state)
        self.assertEqual(gameplay.state_generation, 1)

    def client(self):
        client = LiveWireClient.__new__(LiveWireClient)
        client.gameplay = SimpleNamespace(
            latest_state={"frame": {"contract_version": 9, "can_act": False}},
            socket=Mock(), receive_json=Mock(),
        )
        client.gameplay.socket.gettimeout.return_value = 17
        return client

    def test_wait_restores_timeout_when_transport_fails(self):
        client = self.client()
        client.gameplay.receive_json.side_effect = TimeoutError("no frame")
        with self.assertRaises(TimeoutError):
            client.wait_for(lambda frame: frame["can_act"], timeout=1)
        client.gameplay.socket.settimeout.assert_called_with(17)

    def test_expired_deadline_does_not_start_another_read(self):
        client = self.client()
        with patch("live_wire_client.time.monotonic", side_effect=[10, 12]):
            with self.assertRaises(ProofError):
                client.wait_for(lambda frame: frame["can_act"], timeout=1)
        client.gameplay.receive_json.assert_not_called()
        client.gameplay.socket.settimeout.assert_called_with(17)

    def test_obsolete_frame_contract_is_refused(self):
        client = self.client()
        client.gameplay.latest_state["frame"]["contract_version"] = 8
        with self.assertRaises(ProofError):
            client.frame
