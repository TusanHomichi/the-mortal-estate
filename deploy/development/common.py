"""Private development roots, process execution, and integrity receipts."""
from __future__ import annotations

import hashlib
import json
import os
import select
import subprocess
import time
from pathlib import Path

REPO = Path(__file__).resolve().parents[2]
UNITS = ("tme-development-postgres", "tme-development-server", "tme-development-web")


def run(arguments, *, input=None, env=None, timeout=120, cwd=None):
    result = subprocess.run(list(map(str, arguments)), input=input, text=True,
                            capture_output=True, env=env, timeout=timeout, cwd=cwd)
    if result.returncode:
        # Never echo arguments or stdin: either can carry an operator credential.
        raise RuntimeError(f"{Path(arguments[0]).name} exited {result.returncode}: {result.stderr[-1500:]}")
    return result.stdout.strip()


class SnapshotSession:
    """Holds one exported snapshot open for the lifetime of a backup.

    A backup must describe the same instant it dumps. Reading expectations through
    separate transactions does not do that: a change landing between the dump and the
    reads is recorded as though the dump had contained it, and the drill later reports
    a perfectly good backup as having lost a character.

    PostgreSQL's own mechanism closes the gap. This opens one repeatable-read
    read-only transaction and exports its snapshot; `pg_dump --snapshot` imports it,
    and so does every expectation read (see `operations.at_snapshot`). All of them
    therefore see the one instant.

    The session deliberately does no further work: it exists only to keep the
    exporting transaction open, because the snapshot is released when that
    transaction ends. Reads go through their own bounded `psql` invocations, which is
    more robust than framing a long interactive conversation.
    """

    def __init__(self, site, database, timeout=120):
        self.timeout = timeout
        self.closed = False
        # Binary pipes: the read loop owns its own buffering, so a byte already read
        # from the descriptor cannot hide behind a wrapper while `select` reports the
        # descriptor idle and the deadline expires.
        self._buffer = b""
        self.process = subprocess.Popen(
            list(map(str, [
                site.pg_bin / "psql", "-XqAt", "-v", "ON_ERROR_STOP=1",
                "-h", site.socket, "-p", site.ports["postgres"],
                "-U", site.settings["administrator"], "-d", database])),
            stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.PIPE)
        try:
            # READ ONLY so nothing here can mutate the world it describes, and
            # REPEATABLE READ because a snapshot can only be exported and imported at
            # that level.
            self._send("BEGIN TRANSACTION ISOLATION LEVEL REPEATABLE READ READ ONLY;")
            self._send("SELECT pg_export_snapshot();")
            self.identifier = self._read_line()
        except BaseException as error:
            # The child exists from the moment Popen returns, so a failure to
            # initialize still has to release it. A cleanup problem is attached to the
            # original error rather than replacing it.
            problems = self.close(failed=True)
            if problems:
                raise RuntimeError(f"{error}; releasing the snapshot also failed: "
                                   f"{'; '.join(problems)}") from error
            raise

    def _send(self, statement):
        if self.closed:
            raise RuntimeError("snapshot session is closed")
        try:
            self.process.stdin.write(statement.encode("utf-8") + b"\n")
            self.process.stdin.flush()
        except (BrokenPipeError, ValueError, OSError):
            raise RuntimeError(f"snapshot session ended early: {self._diagnostics()}")

    def _read_line(self):
        """One line, bounded by the deadline rather than by a blocking read."""
        deadline = time.monotonic() + self.timeout
        while b"\n" not in self._buffer:
            remaining = deadline - time.monotonic()
            if remaining <= 0:
                raise RuntimeError(f"snapshot session produced no result within {self.timeout}s")
            ready, _, _ = select.select([self.process.stdout], [], [], remaining)
            if not ready:
                raise RuntimeError(f"snapshot session produced no result within {self.timeout}s")
            chunk = os.read(self.process.stdout.fileno(), 4096)
            if not chunk:
                raise RuntimeError(f"snapshot session ended early: {self._diagnostics()}")
            self._buffer += chunk
        line, _, self._buffer = self._buffer.partition(b"\n")
        return line.decode("utf-8", "replace").strip()

    def _diagnostics(self, limit=500):
        """Whatever the child has already said, without waiting on it."""
        try:
            ready, _, _ = select.select([self.process.stderr], [], [], 1.0)
            if not ready:
                return "no diagnostic output"
            text = os.read(self.process.stderr.fileno(), 4096).decode("utf-8", "replace").strip()
            return text[:limit] or "no diagnostic output"
        except (ValueError, OSError):
            return "diagnostics unavailable"

    def close(self, *, failed=False):
        """End the exporting transaction and finish the child's input.

        Returns a list of cleanup problems; it never raises, so a caller already
        handling an error can report both without losing the original.

        Closing stdin is what ends `psql`'s input loop. COMMIT or ROLLBACK only
        finishes the SQL transaction, so waiting for exit with input still open spends
        the whole timeout and then kills a perfectly healthy client -- a forced
        shutdown that would otherwise be reported as ordinary completion.
        """
        if self.closed:
            return []
        self.closed = True
        problems = []
        try:
            if self.process.poll() is None:
                self.process.stdin.write(("ROLLBACK;" if failed else "COMMIT;").encode("utf-8") + b"\n")
                self.process.stdin.flush()
        except (BrokenPipeError, ValueError, OSError) as error:
            problems.append(f"sending the transaction end failed: {error}")
        try:
            # End of file, so the client stops reading input and can exit on its own.
            self.process.stdin.close()
        except (OSError, ValueError) as error:
            problems.append(f"closing the client's input failed: {error}")
        try:
            self.process.wait(timeout=self.timeout)
        except subprocess.TimeoutExpired:
            # Forced termination recovers from a hang; it is not how shutdown normally
            # ends, and it must not pass silently.
            self.process.kill()
            self.process.wait(timeout=10)
            problems.append(
                f"the client did not exit within {self.timeout}s after its input ended and was killed")
        finally:
            for stream in (self.process.stdout, self.process.stderr):
                try:
                    stream.close()
                except (OSError, ValueError):
                    pass
        if not problems and self.process.returncode != 0:
            problems.append(f"the client exited with status {self.process.returncode}")
        return problems


def write(path: Path, value: str, mode=0o600):
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(value)
    path.chmod(mode)


def document(path: Path, value):
    write(path, json.dumps(value, indent=2) + "\n")


def digest(path: Path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


class Installation:
    def __init__(self, root: Path):
        self.root = root.expanduser().resolve()
        if self.root.is_relative_to(REPO):
            raise ValueError("development state and credentials must live outside the checkout")
        if any(char.isspace() or char in '%"\\' for char in str(self.root)):
            raise ValueError("development root must contain no whitespace, quote, percent or backslash")
        self.config = self.root / "config"
        self.data = self.root / "postgres"
        self.socket = self.root / "socket"
        self.current = self.root / "current"
        self.units = Path.home() / ".config/systemd/user"
        self.settings = json.loads((self.config / "settings.json").read_text()) if (self.config / "settings.json").exists() else None

    @property
    def ports(self):
        return self.settings["ports"]

    @property
    def pg_bin(self):
        return Path(self.settings["postgres_bin"])

    @property
    def origin(self):
        return self.settings["public_origin"]

    @property
    def local_origin(self):
        return f"https://localhost:{self.ports['https']}"

    def pg(self, name, *arguments, input=None, database="tme", timeout=120):
        return run([self.pg_bin / name, "-h", self.socket, "-p", self.ports["postgres"],
                    "-U", self.settings["administrator"], "-d", database, *arguments], input=input, timeout=timeout)

    def sql(self, text, database="tme", quiet=False):
        """Run SQL and return stdout.

        `quiet` suppresses psql command status. Without it a script containing BEGIN
        and COMMIT returns those tags alongside the result, which is not a JSON
        document even though tuples-only formatting hides headers and footers.
        """
        flags = ["-XqAt"] if quiet else ["-XAt"]
        return self.pg("psql", *flags, "-v", "ON_ERROR_STOP=1", input=text, database=database)

    def operator(self, *arguments, input=None, database="tme"):
        from urllib.parse import quote
        url = f"postgresql://{self.settings['administrator']}@localhost:{self.ports['postgres']}/{database}?host={quote(str(self.socket))}&options=-c%20role%3Dtme_owner"
        return run([self.current / "bin/tme-server", *arguments], input=input,
                   env={**os.environ, "DATABASE_URL": url, "TME_BANNED_TERMS_FILE": str(self.config / "banned-terms.txt")})

    def service(self, operation, *names):
        return run(["systemctl", "--user", operation, *[name + ".service" for name in names]])

    def check_release(self, directory=None):
        directory = directory or self.current.resolve()
        if directory.resolve().parent != self.root / "releases":
            raise RuntimeError("active release escapes its installation")
        receipt = json.loads((directory / "release.json").read_text())
        if any(path.is_symlink() for path in directory.rglob("*")):
            raise RuntimeError("release contains a symbolic link")
        actual = {str(path.relative_to(directory)): digest(path) for path in directory.rglob("*")
                  if path.is_file() and path != directory / "release.json"}
        if actual != receipt["files"]:
            raise RuntimeError("release files differ from their integrity receipt")
        return receipt
