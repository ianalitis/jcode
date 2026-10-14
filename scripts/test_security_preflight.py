#!/usr/bin/env python3
"""Local-only preflight regressions. Never print the constructed match or source."""
import os
from pathlib import Path
import shutil
import subprocess
import tempfile
import unittest

SCRIPT = Path(__file__).with_name("security_preflight.sh")


class SecurityPreflightPrivacyTests(unittest.TestCase):
    def run_scan(self, ripgrep, *, missing=False, sensitive=True):
        with tempfile.TemporaryDirectory(dir=os.environ.get("JCODE_SCRATCH_DIR")) as tmp:
            root = Path(tmp)
            (root / "scripts").mkdir()
            shutil.copyfile(SCRIPT, root / "scripts/security_preflight.sh")
            bindir = root / "bin"
            bindir.mkdir()
            for name in ["dirname", "grep", "find"] + (["rg"] if ripgrep else []):
                executable = shutil.which(name)
                self.assertIsNotNone(executable, f"required local test tool: {name}")
                (bindir / name).symlink_to(executable)
            # Fixed metadata-only file inventory, without changing a real index.
            for name, body in [("git", "printf 'sample.txt\\0'"), ("cargo", "exit 1")]:
                stub = bindir / name
                stub.write_text("#!/bin/sh\n" + body + "\n")
                stub.chmod(0o700)
            sentinel = "".join(["AK", "IA", "Z" * 16])
            if not missing:
                (root / "sample.txt").write_text(sentinel if sensitive else "ordinary source\n")
            result = subprocess.run(
                ["/bin/bash", str(root / "scripts/security_preflight.sh")],
                env={**os.environ, "PATH": str(bindir)}, capture_output=True, text=True,
                timeout=10,
            )
            # Assertions report only metadata, even if the guard regresses.
            self.assertFalse(sentinel in result.stdout or sentinel in result.stderr,
                             "scanner disclosed matching source")
            expected_files = ["bin", "scripts"] if missing else ["bin", "sample.txt", "scripts"]
            self.assertEqual(sorted(p.name for p in root.iterdir()), expected_files)
            return result

    def test_matches_fail_without_disclosure(self):
        for ripgrep in [True, False]:
            with self.subTest(ripgrep=ripgrep):
                result = self.run_scan(ripgrep)
                self.assertEqual(result.returncode, 1)
                self.assertIn("potential secret material detected", result.stderr)

    def test_scanner_errors_fail_closed(self):
        for ripgrep in [True, False]:
            with self.subTest(ripgrep=ripgrep):
                result = self.run_scan(ripgrep, missing=True)
                self.assertEqual(result.returncode, 1)
                self.assertIn("secret scan failed to execute", result.stderr)

    def test_clean_source_preserves_success(self):
        for ripgrep in [True, False]:
            with self.subTest(ripgrep=ripgrep):
                result = self.run_scan(ripgrep, sensitive=False)
                self.assertEqual(result.returncode, 0)
                self.assertIn("Security preflight passed", result.stdout)


if __name__ == "__main__":
    unittest.main()
