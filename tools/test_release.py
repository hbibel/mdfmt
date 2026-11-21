import hashlib
import importlib.util
import json
import os
from pathlib import Path
import stat
import subprocess
import sys
import tempfile
import time
import unittest
import zipfile

ROOT = Path(__file__).resolve().parents[1]
TEMP_ROOT = "/tmp/opencode" if Path("/tmp/opencode").is_dir() else None


class NativeReleaseTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.workspace = tempfile.TemporaryDirectory(prefix="mdfmt-release-test-", dir=TEMP_ROOT)
        cls.base = Path(cls.workspace.name)
        cls.archives = None

    @classmethod
    def tearDownClass(cls):
        cls.workspace.cleanup()

    def packages(self):
        if self.archives is None:
            archives = []
            for label in ["first", "second"]:
                output = self.base / label
                result = subprocess.run(
                    [sys.executable, str(ROOT / "tools/release.py"), "--output", str(output), "--build-dir", str(self.base / "build")],
                    cwd=ROOT, capture_output=True, text=True, encoding="utf-8", timeout=1200,
                    env={**os.environ, "SOURCE_DATE_EPOCH": "1700000000"},
                )
                self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
                matches = list(output.glob("*.zip"))
                self.assertEqual(len(matches), 1)
                archives.append(matches[0])
            type(self).archives = archives
        return self.archives

    def test_release_archive_is_reproducible_and_curated(self):
        first, second = self.packages()
        self.assertEqual(first.read_bytes(), second.read_bytes())
        digest = hashlib.sha256(first.read_bytes()).hexdigest()
        self.assertEqual(first.with_suffix(".zip.sha256").read_text(encoding="utf-8"), f"{digest}  {first.name}\n")
        executable = "mdfmt.exe" if os.name == "nt" else "mdfmt"
        with zipfile.ZipFile(first) as archive:
            prefix = first.stem + "/"
            self.assertEqual(set(archive.namelist()), {prefix + name for name in [executable, "README.md", "LICENSE", "Cargo.lock", "BUILD-INFO.json", "THIRD-PARTY-NOTICES.txt", "RUST-LIBRARY-NOTICES.html"]})
            self.assertTrue(all(".." not in Path(name).parts and not name.startswith("/") for name in archive.namelist()))
            self.assertEqual(archive.getinfo(prefix + executable).external_attr >> 16, stat.S_IFREG | 0o755)
            info = json.loads(archive.read(prefix + "BUILD-INFO.json"))
            self.assertEqual(info["version"], "0.1.0")
            self.assertTrue(info["rustc"].startswith("rustc "))
            self.assertIn("host", info)
            self.assertRegex(info["source"]["git_commit"], r"^[0-9a-f]{40}$")
            notices = archive.read(prefix + "THIRD-PARTY-NOTICES.txt").decode("utf-8")
            for name in ["regress", "oxc_resolver", "RE2JS", "Unicode", "fast-glob", "nodejs-built-in-modules"]:
                self.assertIn(name, notices)
            for member in archive.namelist():
                self.assertNotIn(str(ROOT).encode(), archive.read(member), member)

    def test_source_inventory_rejects_environment_paths(self):
        spec = importlib.util.spec_from_file_location("mdfmt_release_guard", ROOT / "tools/release.py")
        release = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(release)
        sandbox = self.base / "guard-inputs"
        (sandbox / "src" / ".env").mkdir(parents=True)
        (sandbox / "Cargo.toml").write_text("fixture", encoding="utf-8")
        (sandbox / "Cargo.lock").write_text("fixture", encoding="utf-8")
        (sandbox / "src" / ".env" / "module.rs").write_text("fixture", encoding="utf-8")
        release.ROOT = sandbox
        try:
            release.source_identity()
        except Exception as error:
            self.assertIsInstance(error, ValueError, "source inventory bypassed its environment-path guard")
            self.assertIn("environment files", str(error))
        else:
            self.fail("source inventory accepted an environment path")

    def test_build_deadline_does_not_wait_for_descendant_pipes(self):
        spec = importlib.util.spec_from_file_location("mdfmt_release_deadline", ROOT / "tools/release.py")
        release = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(release)
        command = "import subprocess,sys,time; subprocess.Popen([sys.executable,'-c','import time; time.sleep(5)']); time.sleep(5)"
        start = time.monotonic()
        with self.assertRaises(subprocess.TimeoutExpired):
            release.run([sys.executable, "-c", command], timeout=0.1)
        self.assertLess(time.monotonic() - start, 3, "build deadline waited on inherited descendant pipes")

    def test_packaged_binary_modes_are_transactional(self):
        archive_path = self.packages()[0]
        extracted = self.base / "extracted"
        with zipfile.ZipFile(archive_path) as archive:
            archive.extractall(extracted)
        binary = extracted / archive_path.stem / ("mdfmt.exe" if os.name == "nt" else "mdfmt")
        binary.chmod(0o755)
        case = self.base / "invocation"
        case.mkdir(exist_ok=True)
        config = case / ".markdownlint-cli2.jsonc"
        document = case / "doc.md"

        def configure(value):
            config.write_text(json.dumps(value), encoding="utf-8")

        def run(*arguments, input_text=None, environment=None):
            return subprocess.run([str(binary), *arguments], cwd=case, input=input_text,
                                  capture_output=True, text=True, encoding="utf-8", timeout=30,
                                  env=environment)

        configure({"config": {"default": False, "MD026": True}})
        original = "# Title!\n"
        document.write_bytes(original.encode())
        checked = run("--check", "doc.md")
        self.assertEqual(checked.returncode, 1, checked.stderr)
        self.assertEqual(checked.stdout, "")
        self.assertEqual(document.read_bytes(), original.encode())
        for arguments in [("doc.md",), ("-",)]:
            result = run(*arguments, input_text=original if arguments == ("-",) else None)
            self.assertEqual(result.returncode, 0, result.stderr)
            self.assertEqual(result.stdout, "# Title\n")
            self.assertEqual(document.read_bytes(), original.encode())
        if os.name != "nt":
            document.chmod(0o640)
        written = run("--write", "doc.md")
        self.assertEqual(written.returncode, 0, written.stderr)
        self.assertEqual(document.read_bytes(), b"# Title\n")
        before = document.stat().st_mtime_ns
        self.assertEqual(run("--write", "doc.md").returncode, 0)
        self.assertEqual(document.stat().st_mtime_ns, before)
        if os.name != "nt":
            self.assertEqual(stat.S_IMODE(document.stat().st_mode), 0o640)
            alias = case / "alias.md"
            alias.symlink_to(document)
            self.assertEqual(run("--write", "alias.md").returncode, 2)
            alias.unlink()
            os.link(document, alias)
            self.assertEqual(run("--write", "doc.md").returncode, 2)
            alias.unlink()
            self.assertEqual(document.read_bytes(), b"# Title\n")

        package = case / "node_modules" / "shared"
        package.mkdir(parents=True, exist_ok=True)
        (package / "package.json").write_text(json.dumps({"name": "shared", "version": "1.0.0", "main": "rules.json"}), encoding="utf-8")
        (package / "rules.json").write_text(json.dumps({"default": False, "MD026": True}), encoding="utf-8")
        pattern = r"^%%%\n[\s\S]*?\n%%%\n"
        configure({"frontMatter": pattern, "config": {"extends": "shared"}})
        original = "%%%\n# Secret!\n%%%\n\n# Body!\n"
        document.write_bytes(original.encode())
        result = run("--write", "doc.md")
        self.assertEqual(result.returncode, 0, result.stderr)
        expected = b"%%%\n# Secret!\n%%%\n\n# Body\n"
        self.assertEqual(document.read_bytes(), expected)
        configure({"frontMatter": r"\p{Letter}", "config": {"default": False, "MD026": True}})
        for mode in ["--check", "--write"]:
            result = run(mode, "doc.md")
            self.assertEqual(result.returncode, 2, result.stderr)
            self.assertEqual(result.stdout, "")
            self.assertIn("frontMatter", result.stderr)
            self.assertEqual(document.read_bytes(), expected)

        configure({"config": {"default": False, "MD013": {"strict": True, "code_block_line_length": 20}}})
        original = b'```json\n{"one": 1, "two": 2, "three": 3}\n```\n'
        document.write_bytes(original)
        empty_path = case / "empty-path"
        empty_path.mkdir(exist_ok=True)
        result = run("--format-code", "--write", "doc.md", environment={**os.environ, "PATH": str(empty_path)})
        self.assertEqual(result.returncode, 1, result.stderr)
        self.assertRegex(result.stderr, r"formatter-(unavailable|skipped)")
        self.assertEqual(document.read_bytes(), original)


if __name__ == "__main__":
    unittest.main()
