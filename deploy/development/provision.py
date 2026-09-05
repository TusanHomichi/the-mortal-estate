"""Provision an isolated private development instance from carried inputs."""
from __future__ import annotations

import copy
import getpass
import json
import os
import secrets
import shutil
import socket
import time
import uuid
from pathlib import Path
from urllib.parse import quote

from common import REPO, UNITS, digest, document, run, write
from services import install_units


def validate_settings(settings):
    if set(settings) != {"schema_version", "world_document", "ports"} or settings["schema_version"] != 1:
        raise ValueError("development configuration shape is not current")
    ports = settings["ports"]
    if set(ports) != {"postgres", "server", "operations", "https"}:
        raise ValueError("configure all four development ports")
    if len(set(ports.values())) != 4 or any(type(port) is not int or not 1024 <= port <= 65535 for port in ports.values()):
        raise ValueError("ports must be distinct unprivileged integers")
    path = (REPO / settings["world_document"]).resolve()
    if not path.is_relative_to(REPO) or not path.is_file():
        raise ValueError("served-world document must be carried in this checkout")


def development_seed(source):
    seed = copy.deepcopy(source)
    players = [actor for actor in seed["actors"] if actor.get("character_id") is not None]
    if len(players) != 1:
        raise ValueError("development bootstrap requires one declared controlled character")
    other = copy.deepcopy(players[0])
    other["id"] = "development_second"
    other["character_id"] = "character:development:second"
    other["location"]["position"]["x"] += 1
    seed["actors"].append(other)
    seed["id"] = "private_development"
    return seed, [players[0]["id"], other["id"]]


def stage_release(site):
    # Only index-carried source is admitted; build output is rebuilt, never a fallback.
    if run(["git", "-C", REPO, "diff", "--name-only"]) or run(["git", "-C", REPO, "ls-files", "--others", "--exclude-standard"]):
        raise RuntimeError("stage source changes in Git before building a development release")
    tree = run(["git", "-C", REPO, "write-tree"])
    revision = run(["git", "-C", REPO, "rev-parse", "HEAD"])
    destination = site.root / "releases" / tree
    if destination.exists():
        site.check_release(destination)
        return destination
    run(["cargo", "build", "--manifest-path", REPO / "Cargo.toml", "--release", "--locked", "--bin", "tme-server"], timeout=1200, cwd=REPO)
    target = Path(os.environ.get("CARGO_TARGET_DIR", REPO / "target"))
    if not target.is_absolute():
        target = REPO / target
    staging = site.root / "releases" / (".stage-" + secrets.token_hex(6))
    staging.mkdir(parents=True)
    try:
        (staging / "bin").mkdir()
        shutil.copy2(target / "release/tme-server", staging / "bin/tme-server")
        for name in run(["git", "-C", REPO, "ls-files", "--", "content"]).splitlines():
            source = REPO / name
            if not source.is_file() or source.is_symlink():
                raise RuntimeError("release content must be regular carried files")
            copied = staging / name
            copied.parent.mkdir(parents=True, exist_ok=True)
            shutil.copyfile(source, copied)
        write(staging / "web/index.html", '<!doctype html><meta charset="utf-8"><title>TME private server</title><h1>Private development server</h1><p>The authoritative service is installed.</p>\n', 0o644)
        if run(["git", "-C", REPO, "write-tree"]) != tree or run(["git", "-C", REPO, "diff", "--name-only"]):
            raise RuntimeError("source changed while building the release")
        document(staging / "release.json", {"schema_version": 1, "source_tree": tree, "base_commit": revision,
                 "contracts": json.loads(run([staging / "bin/tme-server", "contract", "versions"])),
                 "files": {str(path.relative_to(staging)): digest(path) for path in staging.rglob("*") if path.is_file()}})
        staging.rename(destination)
    finally:
        if staging.exists():
            shutil.rmtree(staging)
    return destination


def tls(site):
    directory = site.config / "tls"
    directory.mkdir(exist_ok=True)
    if not (directory / "ca.pem").exists():
        run(["openssl", "req", "-x509", "-newkey", "rsa:3072", "-nodes", "-sha256", "-days", "365",
             "-subj", "/CN=TME private development authority", "-addext", "basicConstraints=critical,CA:TRUE",
             "-addext", "keyUsage=critical,keyCertSign,cRLSign", "-keyout", directory / "ca.key", "-out", directory / "ca.pem"])
    leaf = directory / ("leaf-" + secrets.token_hex(6))
    leaf.mkdir(mode=0o700)
    write(leaf / "server.ext", "subjectAltName=DNS:localhost,IP:127.0.0.1\nbasicConstraints=critical,CA:FALSE\nkeyUsage=critical,digitalSignature,keyEncipherment\nextendedKeyUsage=serverAuth\n")
    run(["openssl", "req", "-new", "-newkey", "rsa:2048", "-nodes", "-sha256", "-subj", "/CN=localhost",
         "-keyout", leaf / "server.key", "-out", leaf / "server.csr"])
    run(["openssl", "x509", "-req", "-in", leaf / "server.csr", "-CA", directory / "ca.pem", "-CAkey", directory / "ca.key",
         "-CAcreateserial", "-days", "30", "-sha256", "-extfile", leaf / "server.ext", "-out", leaf / "server.pem"])
    for path in leaf.iterdir():
        path.chmod(0o600)
    run(["openssl", "verify", "-CAfile", directory / "ca.pem", "-verify_hostname", "localhost", leaf / "server.pem"])
    pointer = directory / ".current-next"
    pointer.unlink(missing_ok=True)
    pointer.symlink_to(leaf.name)
    pointer.replace(directory / "current")


def bootstrap(site, release, accounts):
    path = release / site.settings["world_document"]
    declared = json.loads(path.read_text())
    if declared["schema_version"] != 1 or declared["kind"] != "served_world":
        raise RuntimeError("served-world declaration is not current")
    sources = {key: (path.parent / declared[key]).resolve() for key in ("catalog", "world_template", "simulation_seed")}
    if any(not value.is_relative_to(release) for value in sources.values()):
        raise RuntimeError("served-world input escapes the immutable release")
    seed, actors = development_seed(json.loads(sources["simulation_seed"].read_text()))
    seed_path = site.config / "development-seed.json"
    document(seed_path, seed)
    document(site.config / "bootstrap.json", {"schema_version": 1,
        "catalog": str(sources["catalog"]), "catalog_profile": declared["catalog_profile"], "world_template": str(sources["world_template"]),
        "world": {"facet_id": str(uuid.uuid4()), "key": "private-development", "simulation_seed": str(seed_path), "rng_seed": declared["rng_seed"]},
        "characters": [{"account_id": account["account_id"], "character_id": str(uuid.uuid4()), "slot": 1,
                        "display_name": f"Wayfarer {index + 1}", "actor_id": actors[index]} for index, account in enumerate(accounts)]})
    document(site.config / "seed-sources.json", {str(value.relative_to(release)): digest(value) for value in sources.values()})
    site.operator("bootstrap", "verify", site.config / "bootstrap.json")


def install(site, configuration: Path, denylist: Path, postgres_bin: Path):
    if site.settings is not None or site.root.exists():
        raise RuntimeError("installation root already exists; use explicit lifecycle operations")
    settings = json.loads(configuration.read_text())
    validate_settings(settings)
    if not denylist.is_file() or not denylist.read_bytes().strip():
        raise RuntimeError("the real private denylist must be supplied out of band")
    for port in settings["ports"].values():
        with socket.socket() as probe:
            probe.bind(("127.0.0.1", port))
    for name in UNITS:
        if (site.units / (name + ".service")).exists():
            raise RuntimeError(f"refusing an existing {name} unit")
    site.root.mkdir(parents=True, mode=0o700)
    site.config.mkdir(mode=0o700)
    site.socket.mkdir(mode=0o700)
    settings.update(administrator=getpass.getuser(), postgres_bin=str(postgres_bin.resolve()))
    document(site.config / "settings.json", settings)
    site.settings = settings
    shutil.copyfile(denylist, site.config / "banned-terms.txt")
    (site.config / "banned-terms.txt").chmod(0o600)
    release = stage_release(site)
    site.current.symlink_to(release)
    run([postgres_bin / "initdb", "-D", site.data, "--auth-local=peer", "--auth-host=scram-sha-256", "--encoding=UTF8", "--locale=C.UTF-8"])
    with (site.data / "postgresql.conf").open("a") as output:
        output.write(f"\nport={site.ports['postgres']}\nlisten_addresses='127.0.0.1'\nunix_socket_directories='{site.socket}'\n"
                     "unix_socket_permissions=0700\nmax_connections=40\nshared_buffers='128MB'\nwork_mem='4MB'\nmaintenance_work_mem='64MB'\n")
    write(site.data / "pg_hba.conf", f"local all {settings['administrator']} peer\nlocal all all scram-sha-256\nhost all all 127.0.0.1/32 scram-sha-256\n")
    tls(site)
    install_units(site)
    run(["systemctl", "--user", "daemon-reload"])
    site.service("start", UNITS[0])
    for attempt in range(100):
        try:
            site.sql("SELECT 1", "postgres")
            break
        except RuntimeError:
            if attempt == 99:
                raise
            time.sleep(.1)
    site.sql("CREATE DATABASE tme", "postgres")
    site.sql((REPO / "deploy/production/postgres/18/roles.sql").read_text())
    site.sql("ALTER DATABASE tme OWNER TO tme_owner", "postgres")
    for role, name in (("tme_runtime", "database-url"), ("tme_auth", "auth-database-url")):
        password = secrets.token_hex(32)
        site.sql(f"ALTER ROLE {role} PASSWORD '{password}'")
        write(site.config / "credentials" / name,
              f"postgresql://{role}:{password}@localhost:{site.ports['postgres']}/tme?host={quote(str(site.socket))}\n")
    site.operator("migrate")
    site.sql((REPO / "deploy/production/postgres/18/grants.sql").read_text())
    # Generated test identities only; this is not public enrollment provisioning.
    blocklist = site.config / "synthetic-compromised-passwords.txt"
    write(blocklist, "".join(f"synthetic-compromised-{index:06d}\n" for index in range(10_000)))
    accounts = []
    for number in (1, 2):
        username, password = f"development_{number}", secrets.token_urlsafe(32)
        account_id = site.operator("account", "create", "--username", username, "--display-name", f"Development {number}",
                                   "--compromised-passwords", blocklist, input=f"{password}\n{password}\n").splitlines()[-1]
        accounts.append({"username": username, "password": password, "account_id": account_id})
    document(site.config / "test-accounts.json", accounts)
    bootstrap(site, release, accounts)
    write(site.config / "server.env", f"TME_PUBLIC_LISTEN=127.0.0.1:{site.ports['server']}\nTME_OPS_LISTEN=127.0.0.1:{site.ports['operations']}\n"
          f"TME_PUBLIC_HOST=localhost:{site.ports['https']}\nTME_PUBLIC_ORIGIN={site.origin}\n"
          f"TME_BOOTSTRAP_MANIFEST={site.config}/bootstrap.json\nTME_BANNED_TERMS_FILE={site.config}/banned-terms.txt\nRUST_LOG=info\n")
    site.service("enable", *UNITS)
    site.service("start", UNITS[1], UNITS[2])
