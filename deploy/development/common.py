"""Private development roots, process execution, and integrity receipts."""
from __future__ import annotations

import hashlib
import json
import os
import subprocess
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
        return f"https://localhost:{self.ports['https']}"

    def pg(self, name, *arguments, input=None, database="tme", timeout=120):
        return run([self.pg_bin / name, "-h", self.socket, "-p", self.ports["postgres"],
                    "-U", self.settings["administrator"], "-d", database, *arguments], input=input, timeout=timeout)

    def sql(self, text, database="tme"):
        return self.pg("psql", "-XAt", "-v", "ON_ERROR_STOP=1", input=text, database=database)

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
