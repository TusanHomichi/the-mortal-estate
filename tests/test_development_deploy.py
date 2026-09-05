"""Private deployment isolation and integrity, without touching host services."""
import copy
import json
import sys
import tempfile
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / "deploy/development"))

from common import Installation, digest, document
from operations import verify_backup
from provision import development_seed, validate_settings
from services import install_units


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
        release = self.site.root / "releases/reviewed"
        release.mkdir(parents=True)
        binary = release / "server"
        binary.write_bytes(b"synthetic binary")
        document(release / "release.json", {"files": {"server": digest(binary)}, "contracts": {"storage": {"checkpoint": 1}}})
        self.site.current.symlink_to(release)
        return release

    def test_ports_are_complete_distinct_and_unprivileged(self):
        settings = json.loads((ROOT / "deploy/development/config.example.json").read_text())
        validate_settings(settings)
        for bad in (80, settings["ports"]["postgres"], "18741", True):
            changed = copy.deepcopy(settings)
            changed["ports"]["server"] = bad
            with self.assertRaises(ValueError):
                validate_settings(changed)

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
        self.assertNotEqual(seed["actors"][0]["character_id"], seed["actors"][-1]["character_id"])

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


if __name__ == "__main__":
    unittest.main()
