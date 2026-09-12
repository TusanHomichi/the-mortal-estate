"""Private deployment isolation and integrity, without touching host services."""
import copy
import json
import shutil
import subprocess
import sys
import tempfile
import time
import unittest
from types import SimpleNamespace
from unittest.mock import Mock, patch
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / "deploy/development"))

from common import Installation, SnapshotSession, digest, document
from operations import (
    FENCE_CLEARED_QUERIES,
    FENCE_SNAPSHOT_QUERIES,
    SNAPSHOT_QUERIES,
    backup,
    fence_differences,
    restore_drill,
    state_differences,
    validate_expectations,
    verify_backup,
)
from provision import development_seed, validate_settings
from services import install_units


def synthetic_release(site):
    """A minimal, honestly bound release: one file and its integrity receipt."""
    release = site.root / "releases/reviewed"
    release.mkdir(parents=True)
    binary = release / "server"
    binary.write_bytes(b"synthetic binary")
    document(release / "release.json", {"files": {"server": digest(binary)},
                                        "contracts": {"storage": {"checkpoint": 1}},
                                        "source_tree": "synthetic-tree"})
    site.current.symlink_to(release)
    return release


class PrivateDeployment(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory(prefix="tme-development-test-")
        self.addCleanup(self.temporary.cleanup)
        self.root = Path(self.temporary.name)
        self.site = Installation(self.root / "installation")
        self.site.settings = json.loads((ROOT / "deploy/development/config.example.json").read_text())
        self.site.settings["postgres_bin"] = "/usr/lib/postgresql/18/bin"
        self.site.units = self.root / "units"

    def release(self):
        return synthetic_release(self.site)

    def test_ports_are_complete_distinct_and_unprivileged(self):
        settings = json.loads((ROOT / "deploy/development/config.example.json").read_text())
        validate_settings(settings)
        for bad in (80, settings["ports"]["postgres"], "18741", True):
            changed = copy.deepcopy(settings)
            changed["ports"]["server"] = bad
            with self.assertRaises(ValueError):
                validate_settings(changed)

    def test_origin_and_retired_configuration_are_refused(self):
        settings = json.loads((ROOT / "deploy/development/config.example.json").read_text())
        for origin in ("http://localhost:18743", "https://user@host", "https://host/", "https://host?q=1", "https://host;bad"):
            changed = copy.deepcopy(settings)
            changed["public_origin"] = origin
            with self.assertRaises(ValueError):
                validate_settings(changed)
        settings["schema_version"] = 1
        with self.assertRaises(ValueError):
            validate_settings(settings)

    def test_artwork_copy_is_bound_and_allowlisted(self):
        from artwork import copy_artwork
        packet = self.root / "packet"
        packet.mkdir()
        (packet / "room.png").write_bytes(b"synthetic image")
        (packet / "private-notes.txt").write_text("must not be copied")
        manifest = packet / "pixel-manifest.json"
        manifest.write_text(json.dumps({"assets": [{"file": "room.png", "sha256": digest(packet / "room.png")}]}))
        source = self.root / "source"
        receipt = source / "web/src/play/pixelReceipt.json"
        document(receipt, {"manifest_sha256": digest(manifest)})
        (packet / "body.glb").write_bytes(b"synthetic bound mesh")
        document(source / "web/src/play/dungeon/receipt.json", {"body": {"file": "body.glb", "sha256": digest(packet / "body.glb")}})
        with patch("artwork.REPO", source):
            copy_artwork(packet, self.root / "copied")
            self.assertEqual({p.name for p in (self.root / "copied").iterdir()}, {"pixel-manifest.json", "room.png", "body.glb"})
            (packet / "room.png").write_bytes(b"changed")
            with self.assertRaises(ValueError):
                copy_artwork(packet, self.root / "refused")
            self.assertFalse((self.root / "refused").exists())
            manifest.write_text("{}")
            with self.assertRaises(ValueError):
                copy_artwork(packet, self.root / "wrong-manifest")

    def test_town_packet_refuses_mesh_in_raster_receipt(self):
        from artwork import copy_artwork
        packet = self.root / "packet"
        packet.mkdir()
        (packet / "room.glb").write_bytes(b"synthetic retired mesh")
        manifest = packet / "pixel-manifest.json"
        document(manifest, {"assets": [{"file": "room.glb", "sha256": digest(packet / "room.glb")}]})
        source = self.root / "source"
        document(source / "web/src/play/pixelReceipt.json", {"manifest_sha256": digest(manifest)})
        with patch("artwork.REPO", source), self.assertRaisesRegex(ValueError, "wrong format"):
            copy_artwork(packet, self.root / "refused-mesh")
        self.assertFalse((self.root / "refused-mesh").exists())

    def test_state_cannot_be_installed_inside_source(self):
        with self.assertRaises(ValueError):
            Installation(ROOT / ".workbench/deployment")

    def test_seed_adds_one_controlled_character_without_rewriting_originals(self):
        source = json.loads((ROOT / "content/lands/identity-proof/simulation_seed.json").read_text())
        original = copy.deepcopy(source)
        seed, actors = development_seed(source)
        self.assertEqual(source, original)
        self.assertEqual(seed["actors"][:-1], source["actors"])
        self.assertEqual(len(actors), 2)
        self.assertNotEqual(actors[0], actors[1])
        self.assertEqual(seed["actors"][-1]["location"], source["actors"][0]["location"])
        self.assertEqual(seed["actors"][-1]["carried"]["items"], [])
        self.assertNotEqual(seed["actors"][0]["character_id"], seed["actors"][-1]["character_id"])

    def test_equipped_expedition_seed_retains_unique_item_ownership(self):
        source = json.loads((ROOT / "content/lands/first-expedition/simulation_seed.json").read_text())
        seed, _ = development_seed(source)
        self.assertEqual(seed["actors"][:-1], source["actors"])
        owners = [entry["item_instance_id"] for actor in seed["actors"] for entry in actor["carried"]["items"]]
        self.assertEqual(len(owners), len(set(owners)))
        self.assertEqual(seed["actors"][-1]["location"], source["actors"][0]["location"])

    def test_units_and_frontend_are_isolated_and_bounded(self):
        install_units(self.site)
        units = list(self.site.units.glob("*.service"))
        self.assertEqual(len(units), 3)
        for unit in units:
            text = unit.read_text()
            self.assertIn("MemoryMax=", text)
            self.assertIn("CPUQuota=", text)
            self.assertIn("TasksMax=", text)
            self.assertIn(str(self.site.root), text)
            self.assertNotIn("Requires=postgresql.service", text)
        nginx = (self.site.config / "nginx.conf").read_text()
        self.assertIn("listen 127.0.0.1:", nginx)
        self.assertIn("location /internal/ { return 404; }", nginx)
        self.assertNotIn("0.0.0.0", nginx)
        self.assertIn("absolute_redirect off;", nginx)
        (units[0]).write_text("owned by another project")
        with self.assertRaises(RuntimeError):
            install_units(self.site)

    def test_changed_extra_and_linked_release_files_are_refused(self):
        release = self.release()
        self.site.check_release()
        (release / "server").write_bytes(b"changed")
        with self.assertRaises(RuntimeError):
            self.site.check_release()
        (release / "server").write_bytes(b"synthetic binary")
        extra = release / "extra/release.json"
        extra.parent.mkdir()
        extra.write_text("unbound")
        with self.assertRaises(RuntimeError):
            self.site.check_release()
        extra.unlink()
        (release / "alias").symlink_to(release / "server")
        with self.assertRaises(RuntimeError):
            self.site.check_release()

    def test_backup_refuses_mutation_or_another_storage_contract(self):
        self.release()
        directory = self.site.root / "backups/one"
        directory.mkdir(parents=True)
        dump = directory / "database.dump"
        dump.write_bytes(b"synthetic dump")
        receipt = {"schema_version": 1, "sha256": digest(dump), "storage": {"checkpoint": 1}}
        document(directory / "backup.json", receipt)
        verify_backup(self.site, directory)
        dump.write_bytes(b"changed")
        with self.assertRaises(RuntimeError):
            verify_backup(self.site, directory)
        dump.write_bytes(b"synthetic dump")
        receipt["storage"]["checkpoint"] = 2
        document(directory / "backup.json", receipt)
        with self.assertRaises(RuntimeError):
            verify_backup(self.site, directory)

    def test_a_legacy_backup_stays_verifiable_but_cannot_be_drilled(self):
        """A pre-snapshot backup is still restorable; it just cannot claim preservation."""
        self.release()
        directory = self.site.root / "backups/legacy"
        directory.mkdir(parents=True)
        dump = directory / "database.dump"
        dump.write_bytes(b"synthetic dump")
        document(directory / "backup.json", {"schema_version": 1, "sha256": digest(dump),
                                             "storage": {"checkpoint": 1}})
        verify_backup(self.site, directory)
        with self.assertRaisesRegex(RuntimeError, "records no snapshot"):
            restore_drill(self.site, directory)


class RecordedDatabase:
    """Answers the drill's projections from canned rows, without a database."""

    def __init__(self, state=None, fence=None, cleared=None):
        self.state = state or {}
        self.fence = fence or {}
        self.cleared = cleared or {}

    def sql(self, text, database="tme"):
        for queries, source in ((SNAPSHOT_QUERIES, self.state),
                                (FENCE_SNAPSHOT_QUERIES, self.fence)):
            for name, query in queries.items():
                if text == query:
                    # One aggregate line, exactly as the real query returns.
                    return json.dumps(source.get(name) or [])
        for name, query in FENCE_CLEARED_QUERIES.items():
            if text == query:
                return str(self.cleared.get(name, 0))
        raise AssertionError(f"unexpected query: {text}")


def recorded_state():
    """Representative synthetic rows.

    These stand in for a world holding two seeded characters and one created through
    the normal flow. They are canned rows for comparison coverage only; nothing here
    exercises a real backup or restore.
    """
    characters = [
        {"character_id": "c-seeded-1", "account_id": "a-owner", "slot": 1,
         "display_name": "Wayfarer", "actor_id": "player"},
        {"character_id": "c-seeded-2", "account_id": "a-owner", "slot": 2,
         "display_name": "Second", "actor_id": "player_2"},
        {"character_id": "c-created-3", "account_id": "a-owner", "slot": 3,
         "display_name": "Newly Made", "actor_id": "player_3"},
    ]
    return {
        "accounts": [{"account_id": "a-owner", "username": "operator",
                      "display_name": "Operator", "status": "active"}],
        "characters": characters,
        "facets": [{"facet_id": "f-world", "facet_key": "first_expedition",
                    "catalog_id": "catalog", "profile_id": "profile/first_expedition",
                    "template_id": "template", "content_digest": "ab" * 32,
                    "checkpoint_schema": 3, "facet_revision": 41,
                    "last_server_sequence": 97, "checkpoint_sha256": "cd" * 32}],
    }


def recorded_fence():
    """Pre-fence epochs for the same three characters."""
    return {
        "control_epochs": [{"character_id": "c-created-3", "control_epoch": 0},
                           {"character_id": "c-seeded-1", "control_epoch": 4},
                           {"character_id": "c-seeded-2", "control_epoch": 0}],
        "fence_epoch": [{"restore_fence_epoch": 9}],
    }


#: A PostgreSQL client double that models one coordinated concurrent commit.
#:
#: It answers from two instants: the world as the exported snapshot sees it, and the
#: world after a writer committed while the backup was running. A read carrying
#: `SET TRANSACTION SNAPSHOT` gets the first; any other read gets the second, which
#: is exactly how PostgreSQL answers. Its `pg_dump` writes the pinned instant as the
#: dump, so the receipt and the dump can be compared the way a drill compares them.
DATABASE_DOUBLE = '''#!/usr/bin/env python3
import json, pathlib, sys

views = json.loads(pathlib.Path({views!r}).read_text())
log = pathlib.Path({log!r})
name = pathlib.Path(sys.argv[0]).name

if name == "pg_dump":
    arguments = sys.argv[1:]
    target = pathlib.Path(arguments[arguments.index("--file") + 1])
    snapshot = arguments[arguments.index("--snapshot") + 1] if "--snapshot" in arguments else ""
    view = "pinned" if snapshot else "current"
    target.write_text(json.dumps(views[view]))
    with log.open("a") as handle:
        handle.write("dump " + view + "\\n")
    raise SystemExit(0)

lines = []
for line in sys.stdin:
    lines.append(line)
    if "pg_export_snapshot" in line:
        print("fake-snapshot-1", flush=True)
        lines.extend(sys.stdin)
        break
script = "".join(lines)
if "pg_export_snapshot" in script:
    raise SystemExit(0)
view = "pinned" if "SET TRANSACTION SNAPSHOT" in script else "current"
for query, answers in views["queries"].items():
    if query in script:
        with log.open("a") as handle:
            handle.write(view + " " + answers["name"] + "\\n")
        print(answers[view], flush=True)
        break
else:
    raise SystemExit("the double was asked a query it does not model")
'''


class CoordinatedBackupInstant(unittest.TestCase):
    """A commit that lands during a backup stays out of the receipt and the dump.

    The double models the instant a backup must be bound to. A backup that reads its
    expectations outside the exported snapshot -- the order this replaced, dump first
    and unpinned reads after -- records the later commit and hands the drill a receipt
    describing a dump it does not have, which the drill then reports as a lost
    character.
    """

    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory(prefix="tme-backup-instant-")
        self.addCleanup(self.temporary.cleanup)
        self.root = Path(self.temporary.name)
        self.site = Installation(self.root / "installation")
        self.site.settings = json.loads((ROOT / "deploy/development/config.example.json").read_text())
        self.site.settings["administrator"] = "synthetic"
        self.writes = self.root / "reads.log"
        synthetic_release(self.site)

    def database_double(self, views):
        directory = Path(tempfile.mkdtemp(prefix="tme-database-double-"))
        self.addCleanup(shutil.rmtree, directory, ignore_errors=True)
        (self.root / "views.json").write_text(json.dumps(views))
        script = DATABASE_DOUBLE.format(views=str(self.root / "views.json"), log=str(self.writes))
        for name in ("psql", "pg_dump"):
            path = directory / name
            path.write_text(script, encoding="utf-8")
            path.chmod(0o755)
        return directory

    def views(self, pinned, current):
        """Both instants, and the answer the double gives for every expectation query."""
        fence, fence_now = recorded_fence(), copy.deepcopy(recorded_fence())
        fence_now["control_epochs"].append({"character_id": "c-concurrent", "control_epoch": 0})
        before, after = {**pinned, **fence}, {**current, **fence_now}
        queries = {query: {"name": section, "pinned": json.dumps(before[section]),
                           "current": json.dumps(after[section])}
                   for section, query in {**SNAPSHOT_QUERIES, **FENCE_SNAPSHOT_QUERIES}.items()}
        return {"pinned": before, "current": after, "queries": queries}

    def test_a_concurrent_commit_stays_out_of_the_receipt_and_the_dump(self):
        pinned = recorded_state()
        current = copy.deepcopy(pinned)
        current["characters"].append({"character_id": "c-concurrent", "account_id": "a-owner",
                                      "slot": 4, "display_name": "Committed Mid-Backup",
                                      "actor_id": "player_4"})
        self.site.settings["postgres_bin"] = self.database_double(self.views(pinned, current))

        directory = backup(self.site)
        receipt = json.loads((directory / "backup.json").read_text())
        dumped = json.loads((directory / "database.dump").read_text())

        # Exactly one read per expectation, each pinned to the exported snapshot,
        # and one dump. An unpinned read would log "current" here.
        self.assertEqual(self.writes.read_text().splitlines(),
                         [f"pinned {section}" for section in
                          (*SNAPSHOT_QUERIES, *FENCE_SNAPSHOT_QUERIES)] + ["dump pinned"])
        self.assertEqual(receipt["snapshot"], pinned)
        self.assertEqual(receipt["fence"], recorded_fence())
        # The dump carries the same instant the receipt describes, which is what the
        # drill compares; only the fence's intentional mutations differ from it.
        self.assertEqual({name: dumped[name] for name in SNAPSHOT_QUERIES}, pinned)
        self.assertEqual({name: dumped[name] for name in FENCE_SNAPSHOT_QUERIES}, recorded_fence())
        self.assertEqual(state_differences(receipt["snapshot"], dumped), [])
        # The case is not vacuous: had the reads seen the writer's commit, the
        # comparison the drill performs would have named the character it cannot find.
        self.assertTrue(state_differences(receipt["snapshot"], current))


class RestoreReceiptShape(unittest.TestCase):
    """A malformed receipt is refused, and never quietly checks less."""

    def test_a_complete_receipt_is_usable(self):
        self.assertEqual(validate_expectations(recorded_state(), recorded_fence()), [])

    def test_a_missing_fence_epoch_is_refused(self):
        """The increment must not become optional just because the section is gone."""
        fence = recorded_fence()
        del fence["fence_epoch"]
        problems = validate_expectations(recorded_state(), fence)
        self.assertTrue(any("fence_epoch" in problem for problem in problems), problems)

    def test_an_empty_fence_epoch_is_refused(self):
        fence = recorded_fence()
        fence["fence_epoch"] = []
        problems = validate_expectations(recorded_state(), fence)
        self.assertTrue(any("fence epochs" in problem for problem in problems), problems)

    def test_a_missing_preservation_section_is_refused(self):
        state = recorded_state()
        del state["characters"]
        problems = validate_expectations(state, recorded_fence())
        self.assertTrue(any("characters" in problem for problem in problems), problems)

    def test_more_than_one_world_is_refused(self):
        state = recorded_state()
        state["facets"].append(dict(state["facets"][0], facet_id="f-second"))
        problems = validate_expectations(state, recorded_fence())
        self.assertTrue(any("2 worlds" in problem for problem in problems), problems)

    def test_disagreeing_character_identities_are_refused(self):
        """Preserved and epoch sections must describe the same characters."""
        fence = recorded_fence()
        fence["control_epochs"].append({"character_id": "c-ghost", "control_epoch": 0})
        problems = validate_expectations(recorded_state(), fence)
        self.assertTrue(any("disagree" in problem for problem in problems), problems)

    def test_a_non_integer_epoch_is_refused(self):
        fence = recorded_fence()
        fence["control_epochs"][0]["control_epoch"] = "zero"
        problems = validate_expectations(recorded_state(), fence)
        self.assertTrue(any("not a storable" in problem for problem in problems), problems)

    def test_a_boolean_epoch_is_refused(self):
        """`True` is an `int`, so an isinstance check would accept it as one."""
        for value in (True, False):
            fence = recorded_fence()
            fence["control_epochs"][0]["control_epoch"] = value
            problems = validate_expectations(recorded_state(), fence)
            self.assertTrue(any("not a storable" in problem for problem in problems),
                            (value, problems))

    def test_a_negative_or_oversized_epoch_is_refused(self):
        for value in (-1, 2 ** 63):
            fence = recorded_fence()
            fence["control_epochs"][0]["control_epoch"] = value
            problems = validate_expectations(recorded_state(), fence)
            self.assertTrue(any("not a storable" in problem for problem in problems),
                            (value, problems))
            fence = recorded_fence()
            fence["fence_epoch"] = [{"restore_fence_epoch": value}]
            problems = validate_expectations(recorded_state(), fence)
            self.assertTrue(any("fence epoch" in problem for problem in problems), (value, problems))

    def test_a_row_that_is_not_an_object_is_refused(self):
        """Rows are inspected as objects only once that has been confirmed."""
        for section in ("characters", "accounts", "facets"):
            state = recorded_state()
            state[section].append("not a row")
            problems = validate_expectations(state, recorded_fence())
            self.assertTrue(any(section in problem and "not an object" in problem
                                for problem in problems), (section, problems))

    def test_a_row_without_an_identity_is_refused(self):
        state = recorded_state()
        del state["characters"][0]["character_id"]
        problems = validate_expectations(state, recorded_fence())
        self.assertTrue(any("without a character identity" in problem for problem in problems),
                        problems)

    def test_a_repeated_identity_is_refused(self):
        """A set comparison cannot see a duplicate, and a duplicate hides a row."""
        state = recorded_state()
        state["characters"][1]["character_id"] = state["characters"][0]["character_id"]
        problems = validate_expectations(state, recorded_fence())
        self.assertTrue(any("repeats a character identity" in problem for problem in problems),
                        problems)

    def test_a_repeated_epoch_record_is_refused(self):
        fence = recorded_fence()
        fence["control_epochs"][1]["character_id"] = fence["control_epochs"][0]["character_id"]
        problems = validate_expectations(recorded_state(), fence)
        self.assertTrue(any("repeats a character identity" in problem for problem in problems),
                        problems)


class RestorePreservation(unittest.TestCase):
    """The drill compares identities and durable state, not a count."""

    def test_an_unchanged_restore_is_accepted(self):
        state = recorded_state()
        self.assertEqual(state_differences(state, copy.deepcopy(state)), [])

    def test_a_same_count_substitution_is_rejected(self):
        """The count this replaced could not see a swapped identity at all."""
        expected, actual = recorded_state(), recorded_state()
        actual["characters"][2]["character_id"] = "c-impostor"
        actual["characters"][2]["display_name"] = "Impostor"
        self.assertEqual(len(actual["characters"]), len(expected["characters"]))
        differences = state_differences(expected, actual)
        self.assertTrue(any("c-created-3" in row and "lost" in row for row in differences), differences)
        self.assertTrue(any("c-impostor" in row and "gained" in row for row in differences), differences)

    def test_altered_durable_state_is_rejected(self):
        """A facet digest is durable state; a count says nothing about it."""
        expected, actual = recorded_state(), recorded_state()
        actual["facets"][0]["checkpoint_sha256"] = "ef" * 32
        differences = state_differences(expected, actual)
        self.assertTrue(any("facets" in row for row in differences), differences)

    def test_ownership_and_saved_slot_are_part_of_the_comparison(self):
        expected, actual = recorded_state(), recorded_state()
        actual["characters"][2]["account_id"] = "a-somebody-else"
        actual["characters"][2]["slot"] = 7
        differences = state_differences(expected, actual)
        self.assertTrue(any("a-somebody-else" in row for row in differences), differences)

    def test_a_lost_character_is_named_not_counted(self):
        expected, actual = recorded_state(), recorded_state()
        actual["characters"].pop()
        differences = state_differences(expected, actual)
        self.assertEqual(len(differences), 1, differences)
        self.assertIn("c-created-3", differences[0])


class RestoreFenceExpectations(unittest.TestCase):
    """Fence changes are asserted deliberately, never mistaken for lost state."""

    def fence(self, epochs, fence_epoch):
        return {"control_epochs": [{"character_id": key, "control_epoch": value}
                                   for key, value in sorted(epochs.items())],
                "fence_epoch": [{"restore_fence_epoch": fence_epoch}]}

    def test_the_recorded_fence_is_accepted(self):
        expected = self.fence({"c-1": 4, "c-2": 0}, 9)
        site = RecordedDatabase(fence=self.fence({"c-1": 5, "c-2": 1}, 10))
        self.assertEqual(fence_differences(expected, site, "scratch"), [])

    def test_a_missing_epoch_increment_is_rejected(self):
        expected = self.fence({"c-1": 4}, 9)
        site = RecordedDatabase(fence=self.fence({"c-1": 4}, 10))
        differences = fence_differences(expected, site, "scratch")
        self.assertTrue(any("control_epoch is 4, expected 5" in row for row in differences), differences)

    def test_a_surviving_session_or_ticket_is_rejected(self):
        expected = self.fence({"c-1": 4}, 9)
        site = RecordedDatabase(fence=self.fence({"c-1": 5}, 10),
                                cleared={"unrevoked_sessions": 1, "unconsumed_tickets": 2})
        differences = fence_differences(expected, site, "scratch")
        self.assertTrue(any("unrevoked_sessions is 1" in row for row in differences), differences)
        self.assertTrue(any("unconsumed_tickets is 2" in row for row in differences), differences)

    def test_a_character_appearing_across_the_fence_is_rejected(self):
        expected = self.fence({"c-1": 4}, 9)
        site = RecordedDatabase(fence=self.fence({"c-1": 5, "c-extra": 1}, 10))
        differences = fence_differences(expected, site, "scratch")
        self.assertTrue(any("c-extra appeared" in row for row in differences), differences)


class SnapshotShutdown(unittest.TestCase):
    """Ending the transaction must finish the client's input, not kill the client.

    A client double stands in for `psql`. Requiring only that no process survives
    would let a timeout-then-kill pass, so these cases distinguish an exit the client
    chose from one that was forced.
    """

    def double(self, body):
        directory = Path(tempfile.mkdtemp(prefix="tme-psql-double-"))
        self.addCleanup(shutil.rmtree, directory, ignore_errors=True)
        script = directory / "psql"
        script.write_text("#!/usr/bin/env python3\n" + body, encoding="utf-8")
        script.chmod(0o755)
        return SimpleNamespace(pg_bin=directory, socket=Path("/tmp"),
                               ports={"postgres": 1}, settings={"administrator": "x"})

    def session(self, body, timeout=5):
        return SnapshotSession(self.double(body), "scratch", timeout=timeout)

    GRACEFUL = (
        "import sys\n"
        "for line in sys.stdin:\n"
        "    if 'pg_export_snapshot' in line:\n"
        "        print('fake-snapshot-1', flush=True)\n"
    )
    STUBBORN = GRACEFUL + "import time\ntime.sleep(60)\n"
    DIES = "import sys\nsys.exit(0)\n"

    def test_the_client_exits_on_its_own(self):
        """Closing input is what ends the loop; the transaction command alone does not."""
        session = self.session(self.GRACEFUL)
        started = time.monotonic()
        problems = session.close()
        self.assertEqual(problems, [])
        self.assertEqual(session.process.returncode, 0)
        self.assertLess(time.monotonic() - started, 3.0,
                        "a healthy client must not be waited on until the timeout")

    def test_cleanup_after_a_failure_is_also_graceful(self):
        session = self.session(self.GRACEFUL)
        problems = session.close(failed=True)
        self.assertEqual(problems, [])
        self.assertEqual(session.process.returncode, 0)

    def test_a_client_that_will_not_exit_is_reported_not_ignored(self):
        session = self.session(self.STUBBORN, timeout=1)
        problems = session.close()
        self.assertTrue(any("did not exit" in problem for problem in problems), problems)
        self.assertEqual(session.process.returncode, -9)

    def test_a_client_that_already_died_is_reported(self):
        session = self.session(self.GRACEFUL)
        session.process.kill()
        session.process.wait()
        problems = session.close(failed=True)
        self.assertTrue(any("exited with status" in problem for problem in problems), problems)

    def test_closing_twice_is_harmless(self):
        session = self.session(self.GRACEFUL)
        self.assertEqual(session.close(), [])
        self.assertEqual(session.close(), [])

    def test_a_failed_initialization_releases_the_child(self):
        """The constructor owns the child even when it cannot finish setting up."""
        with self.assertRaises(RuntimeError) as caught:
            self.session(self.DIES)
        self.assertIn("ended early", str(caught.exception))

    def test_a_kill_that_cannot_be_delivered_is_reported_not_raised(self):
        """A client that died between the timeout and the kill is a cleanup problem.

        Forced termination is the recovery path, so a failure inside it must not
        escape `close`: the caller is often already handling a failure of its own and
        keeps that original failure only because this one is returned with it.
        """
        session = self.session(self.STUBBORN, timeout=1)
        self.addCleanup(session.process.kill)
        with patch.object(session.process, "kill", side_effect=ProcessLookupError("already gone")):
            problems = session.close()
        self.assertTrue(any("did not exit" in problem for problem in problems), problems)
        self.assertTrue(any("killing it failed" in problem for problem in problems), problems)

    def test_a_killed_client_that_still_will_not_exit_is_reported(self):
        """A second wait that also expires is reported rather than raised."""
        session = self.session(self.STUBBORN, timeout=1)
        self.addCleanup(lambda: subprocess.Popen.wait(session.process, timeout=10))
        self.addCleanup(subprocess.Popen.kill, session.process)
        with patch.object(session.process, "kill", return_value=None), \
                patch.object(session.process, "wait",
                             side_effect=subprocess.TimeoutExpired("psql", 10)):
            problems = session.close()
        self.assertTrue(any("did not exit" in problem for problem in problems), problems)
        self.assertTrue(any("within 10s of being killed" in problem for problem in problems), problems)

    def test_a_failed_initialization_keeps_its_original_failure_when_cleanup_fails(self):
        """The constructor's own failure survives a forced termination that fails."""
        with popen_whose_kill_fails(self):
            with self.assertRaises(RuntimeError) as caught:
                SnapshotSession(self.double("import time\ntime.sleep(60)\n"), "scratch", timeout=1)
        self.assertIn("produced no result", str(caught.exception))
        self.assertIn("killing it failed", str(caught.exception))
        self.assertIn("releasing the snapshot also failed", str(caught.exception))


def popen_whose_kill_fails(case):
    """Real clients, whose forced termination cannot be delivered.

    The child is a real process, because a shutdown path that only appears to kill
    something is the defect these cases exist to catch. Only the kill is injected: an
    unkillable client cannot be produced portably.
    """
    real_popen = subprocess.Popen
    started = []

    def popen(*arguments, **keywords):
        process = real_popen(*arguments, **keywords)
        started.append(process)
        process.kill = Mock(side_effect=ProcessLookupError("already gone"))
        return process

    case.addCleanup(
        lambda: [real_popen.kill(process) for process in started if process.poll() is None])
    return patch("subprocess.Popen", side_effect=popen)


class BackupFailureReporting(unittest.TestCase):
    """A backup reports its own failure first, however cleanup went."""

    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory(prefix="tme-backup-failure-")
        self.addCleanup(self.temporary.cleanup)
        self.root = Path(self.temporary.name)
        self.pg_bin = self.root / "bin"
        self.pg_bin.mkdir()
        # A `psql` that exports a snapshot, then waits forever without answering.
        script = self.pg_bin / "psql"
        script.write_text(
            "#!/usr/bin/env python3\n"
            "import sys, time\n"
            "for line in sys.stdin:\n"
            "    if 'pg_export_snapshot' in line:\n"
            "        print('fake-snapshot-1', flush=True)\n"
            "time.sleep(60)\n",
            encoding="utf-8")
        script.chmod(0o755)

        def check_release(directory=None):
            return {"source_tree": "synthetic-tree", "contracts": {"storage": {"checkpoint": 1}}}

        def failing_read(text, database="tme", quiet=False):
            raise RuntimeError("the expectation read failed")

        def unexpected_dump(name, *arguments, **keywords):
            raise AssertionError("the dump must not run once the reads have failed")

        self.site = SimpleNamespace(
            root=self.root / "installation", pg_bin=self.pg_bin, socket=Path("/tmp"),
            ports={"postgres": 1}, settings={"administrator": "x"},
            check_release=check_release, sql=failing_read, pg=unexpected_dump)

    def test_the_original_failure_survives_a_forced_cleanup_that_fails(self):
        """Losing the cause would turn a real defect into an unexplained crash."""
        with popen_whose_kill_fails(self), \
                patch("operations.SnapshotSession",
                      lambda site, database: SnapshotSession(site, database, timeout=1)):
            with self.assertRaises(RuntimeError) as caught:
                backup(self.site)
        message = str(caught.exception)
        self.assertIn("the expectation read failed", message)
        self.assertIn("releasing the snapshot also failed", message)
        self.assertIn("did not exit", message)
        self.assertIn("killing it failed", message)
        self.assertIsInstance(caught.exception.__cause__, RuntimeError)
        self.assertEqual(list((self.site.root / "backups").rglob("database.dump")), [],
                         "a backup that failed must publish no dump")


class DrillDatabase(RecordedDatabase):
    """A recorded database that a real drill can run against.

    The restore and fence commands are no-ops: what these cases exercise is the
    drill's own comparison and its cleanup, not PostgreSQL. The cleanup can be made
    to fail, which is the boundary a drill's `finally` used to lose.
    """

    def __init__(self, root, cleanup_error=None, **keywords):
        super().__init__(**keywords)
        self.root = root
        self.settings = {"administrator": "x", "postgres_bin": "/nonexistent"}
        self.socket = Path("/tmp")
        self.ports = {"postgres": 1}
        self.cleanup_error = cleanup_error
        self.created = []
        self.dropped = []

    def check_release(self, directory=None):
        return {"source_tree": "synthetic", "contracts": {"storage": {"checkpoint": 1}}}

    def pg(self, name, *arguments, **keywords):
        return ""

    def operator(self, *arguments, **keywords):
        return ""

    def sql(self, text, database="tme", quiet=False):
        if text.startswith("CREATE DATABASE"):
            self.created.append(text)
            return ""
        if text.startswith("DROP DATABASE"):
            if self.cleanup_error is not None:
                raise self.cleanup_error
            self.dropped.append(text)
            return ""
        return super().sql(text, database)


class DrillCleanupReporting(unittest.TestCase):
    """A drill keeps its own failure when dropping its scratch database fails.

    The scratch database is the drill's, and a leak is a real operational problem:
    whoever has to remove it needs its name, and whoever has to fix the drill needs
    the comparison failure rather than a cleanup traceback in its place.
    """

    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory(prefix="tme-drill-cleanup-")
        self.addCleanup(self.temporary.cleanup)
        self.root = Path(self.temporary.name)
        self.site = DrillDatabase(self.root / "installation", state=recorded_state(),
                                  fence=recorded_fence())

    def backup(self, state, fence):
        directory = self.site.root / "backups/drill"
        directory.mkdir(parents=True)
        dump = directory / "database.dump"
        dump.write_bytes(b"synthetic dump")
        document(directory / "backup.json", {"schema_version": 2, "sha256": digest(dump),
                                             "storage": {"checkpoint": 1},
                                             "snapshot": state, "fence": fence})
        return directory

    def fenced(self, increment=1):
        fence = recorded_fence()
        fence["control_epochs"] = [dict(row, control_epoch=row["control_epoch"] + increment)
                                   for row in fence["control_epochs"]]
        fence["fence_epoch"] = [{"restore_fence_epoch":
                                 fence["fence_epoch"][0]["restore_fence_epoch"] + increment}]
        return fence

    def test_a_failing_comparison_survives_a_cleanup_that_fails(self):
        """The lost character is the finding; the surviving database is the follow-up."""
        expected = recorded_state()
        restored = copy.deepcopy(expected)
        restored["characters"].pop()
        self.site.state = restored
        self.site.cleanup_error = RuntimeError("database is being used by prepared transactions")
        with self.assertRaises(RuntimeError) as caught:
            restore_drill(self.site, self.backup(expected, recorded_fence()))
        message = str(caught.exception)
        self.assertIn("did not retain its backup", message)
        self.assertIn("c-created-3", message)
        self.assertIn("releasing the drill database also failed", message)
        self.assertIn("database is being used by prepared transactions", message)
        self.assertTrue(any(name.split()[2] in message for name in self.site.created), message)
        self.assertIsInstance(caught.exception.__cause__, RuntimeError)
        self.assertIn("did not retain its backup", str(caught.exception.__cause__))

    def test_a_cleanup_timeout_is_reported_alongside_the_failure(self):
        """A cleanup can time out; that is still a problem, not a replacement."""
        expected = recorded_state()
        restored = copy.deepcopy(expected)
        restored["facets"][0]["checkpoint_sha256"] = "ef" * 32
        self.site.state = restored
        self.site.cleanup_error = subprocess.TimeoutExpired("psql", 120)
        with self.assertRaises(RuntimeError) as caught:
            restore_drill(self.site, self.backup(expected, recorded_fence()))
        message = str(caught.exception)
        self.assertIn("did not retain its backup", message)
        self.assertIn("was not dropped", message)

    def test_a_verified_drill_that_cannot_clean_up_fails(self):
        """A drill that leaves its scratch database behind has not finished."""
        state = recorded_state()
        self.site.fence = {"control_epochs": self.fenced()["control_epochs"],
                           "fence_epoch": self.fenced()["fence_epoch"]}
        self.site.cleanup_error = RuntimeError("injected DROP failure")
        with self.assertRaises(RuntimeError) as caught:
            restore_drill(self.site, self.backup(state, recorded_fence()))
        self.assertIn("verified the restore but", str(caught.exception))
        self.assertIn("injected DROP failure", str(caught.exception))

    def test_a_clean_drill_verifies_and_drops_its_scratch_database(self):
        state = recorded_state()
        self.site.fence = {"control_epochs": self.fenced()["control_epochs"],
                           "fence_epoch": self.fenced()["fence_epoch"]}
        report = restore_drill(self.site, self.backup(state, recorded_fence()))
        self.assertTrue(report["fenced_and_verified"])
        self.assertEqual(report["restored_characters"], [row["character_id"]
                                                         for row in state["characters"]])
        self.assertEqual(len(self.site.created), 1)
        self.assertEqual(len(self.site.dropped), 1)
        self.assertIn(self.site.created[0].split()[2], self.site.dropped[0])


if __name__ == "__main__":
    unittest.main()
