"""Real HTTP logout across completed and racing durable socket teardown."""
from __future__ import annotations

import select
import subprocess
import threading
import time

from live_server_harness import ProofError, run
from live_wire_client import LiveWireClient


def _until(predicate, reason: str) -> None:
    deadline = time.monotonic() + 4
    while not predicate():
        if time.monotonic() >= deadline:
            raise ProofError(reason)
        time.sleep(0.01)


def prove_disconnected_logout(server) -> None:
    """An external session row lock puts logout's auth snapshot before detach.

    This is a database scheduling barrier, never a product retry or delay.
    PostgreSQL's lock waiter and committed world revision prove the ordering.
    """
    def sql(query):
        return run(["psql", server.database_url, "-XAt", "-v", "ON_ERROR_STOP=1", "-c", query]).strip()

    for racing in (False, True):
        client = LiveWireClient(server).__enter__()
        client.session.ticket()  # A genuinely unused ticket must be revoked too.
        token = client.session.token
        before = sql("SELECT facet_revision FROM tme.facets")
        locker = None
        logout_thread = None
        failures = []

        def logout():
            try:
                client.session.logout()
            except Exception as error:
                failures.append(error)

        try:
            if racing:
                locker = subprocess.Popen(["psql", server.database_url, "-XAt", "-v", "ON_ERROR_STOP=1"],
                                          stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.PIPE)
                locker.stdin.write(b"BEGIN; SELECT session_id FROM tme.sessions WHERE revoked_at IS NULL FOR UPDATE;\\echo LOCKED\n")
                locker.stdin.flush()
                received = bytearray()
                deadline = time.monotonic() + 4
                while b"LOCKED\n" not in received:
                    remaining = deadline - time.monotonic()
                    if remaining <= 0 or not select.select([locker.stdout], [], [], remaining)[0]:
                        raise ProofError("session row lock did not become ready")
                    received.extend(locker.stdout.read1(4096))
                logout_thread = threading.Thread(target=logout, daemon=True)
                logout_thread.start()
                _until(lambda: sql("SELECT count(*) FROM pg_stat_activity WHERE wait_event_type='Lock' "
                                   "AND query LIKE 'SELECT session_id,account_id,csrf_digest%'") != "0",
                       "logout did not wait behind the session row lock")
            client.gameplay.close()
            _until(lambda: sql("SELECT facet_revision FROM tme.facets") != before,
                   "socket teardown did not commit its presence checkpoint")
        finally:
            if locker is not None:
                try:
                    locker.communicate(b"COMMIT;\n\\q\n", timeout=5)
                except subprocess.TimeoutExpired:
                    locker.kill(); locker.communicate()
            if logout_thread is not None:
                logout_thread.join(timeout=30)
                if logout_thread.is_alive():
                    raise ProofError("racing logout did not finish")
            client.gameplay.close()
        if not racing:
            logout()
        if failures:
            raise ProofError(f"disconnected logout failed (racing={racing}): {failures[0]}\n{server.log_tail()}")
        client.public.request("POST", "/v4/session", body={}, token=token, expected=(401,))
        active = sql("SELECT count(*) FROM tme.socket_tickets t JOIN tme.sessions s USING (session_id) "
                     "WHERE s.revoked_at IS NOT NULL AND t.consumed_at IS NULL")
        if active != "0":
            raise ProofError("logout left unused tickets for a revoked session")
        print(f"disconnected logout: HTTP 204, token revoked, unused tickets removed; racing={racing}")
