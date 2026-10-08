#!/usr/bin/env python3
"""Offline regression tests for the warning gate using a synthetic Cargo executable."""

import os
from pathlib import Path
import shutil
import subprocess
import tempfile
import unittest


class WarningBudgetTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory(dir=os.environ.get("JCODE_SCRATCH_DIR"))
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        scripts = self.root / "scripts"
        scripts.mkdir()
        self.script = scripts / "check_warning_budget.sh"
        shutil.copyfile(Path(__file__).with_name(self.script.name), self.script)
        self.baseline = scripts / "warning_budget.txt"
        self.baseline.write_text("0\n")
        bin_dir = self.root / "bin"
        bin_dir.mkdir()
        cargo = bin_dir / "cargo"
        cargo.write_text(
            '#!/bin/sh\n'
            ': > "$FAKE_CARGO_CALLED"\n'
            'if [ -n "$FAKE_CARGO_OUTPUT_FILE" ]; then\n'
            '  cat "$FAKE_CARGO_OUTPUT_FILE" >&2\n'
            'else\n'
            '  printf "%s" "$FAKE_CARGO_OUTPUT" >&2\n'
            'fi\n'
            'exit "$FAKE_CARGO_EXIT"\n'
        )
        cargo.chmod(0o700)
        self.called = self.root / "cargo-called"
        self.env = {
            "PATH": f"{bin_dir}:/usr/bin:/bin",
            "HOME": str(self.root),
            "TMPDIR": str(self.root),
            "FAKE_CARGO_CALLED": str(self.called),
        }

    def run_gate(self, output="", exit_code=0, *args):
        return subprocess.run(
            ["/bin/bash", str(self.script), *args],
            env={**self.env, "FAKE_CARGO_OUTPUT": output, "FAKE_CARGO_EXIT": str(exit_code)},
            capture_output=True,
            text=True,
            timeout=10,
        )

    def test_clean_compile_passes(self):
        result = self.run_gate()
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertIn("current=0 baseline=0", result.stdout)

    def test_warning_over_budget_fails(self):
        result = self.run_gate("warning: synthetic warning\n")
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("Warning budget exceeded", result.stderr)
        # The gate must name what it counted: the count alone cannot be reproduced
        # locally on a warm tree, so the CI log is the only place the evidence exists.
        self.assertIn("warning: synthetic warning", result.stderr)

    def test_over_budget_output_is_bounded_and_counted(self):
        output = "".join(f"warning: synthetic {index}\n" for index in range(25))
        result = self.run_gate(output)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("warning: synthetic 19", result.stderr)
        self.assertNotIn("warning: synthetic 20", result.stderr)
        self.assertIn("and 5 more", result.stderr)

    def test_warnings_at_budget_pass(self):
        self.baseline.write_text("1\n")
        result = self.run_gate("warning: synthetic warning\n")
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertIn("current=1 baseline=1", result.stdout)

    def test_compiler_failure_preserves_status_and_diagnostics(self):
        for output in ("error: synthetic failure\n", "warning: synthetic warning\nerror: synthetic failure\n"):
            with self.subTest(output=output):
                result = self.run_gate(output, 17)
                self.assertEqual(result.returncode, 17)
                self.assertIn("error: synthetic failure", result.stderr)
                self.assertNotIn("Warning budget OK", result.stdout)

    def test_failed_compile_cannot_update_baseline(self):
        self.baseline.write_text("2\n")
        result = self.run_gate("warning: synthetic warning\nerror: synthetic failure\n", 17, "--update")
        self.assertEqual(self.baseline.read_text(), "2\n")
        self.assertEqual(result.returncode, 17)
        self.assertIn("error: synthetic failure", result.stderr)

    def test_successful_compile_can_update_fixture_baseline(self):
        result = self.run_gate("warning: synthetic warning\n", 0, "--update")
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(self.baseline.read_text(), "1\n")

    def test_invalid_baseline_fails(self):
        for args in ((), ("--update",)):
            for baseline in ("invalid\n", "-1\n", "1\n2\n", "1\n\n", "1\n\n\n", "", "1\x00\n", "1234567890123456789\n"):
                with self.subTest(args=args, baseline=baseline):
                    self.baseline.write_text(baseline)
                    self.called.unlink(missing_ok=True)
                    result = self.run_gate("warning: should not run\n", 0, *args)
                    self.assertNotEqual(result.returncode, 0)
                    self.assertIn("invalid warning baseline", result.stderr)
                    self.assertEqual(self.baseline.read_text(), baseline)
                    self.assertFalse(self.called.exists(), "invalid baseline must not invoke Cargo")

    def test_single_decimal_line_accepts_optional_final_newline(self):
        for baseline in ("1", "1\n", "0001\n"):
            with self.subTest(baseline=baseline):
                self.baseline.write_text(baseline)
                result = self.run_gate("warning: synthetic warning\n")
                self.assertEqual(result.returncode, 0, result.stderr)
                self.assertIn("current=1 baseline=1", result.stdout)
                self.assertEqual(self.baseline.read_text(), baseline)

    def test_missing_cargo_fails_without_changing_baseline(self):
        # Only the script's read-only preflight utilities are visible.
        bin_dir = self.root / "without-cargo"
        bin_dir.mkdir()
        for utility in ("dirname", "tr", "cat", "awk", "wc"):
            (bin_dir / utility).symlink_to(shutil.which(utility))
        self.env["PATH"] = str(bin_dir)
        result = self.run_gate("", 0, "--update")
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("cargo not found", result.stderr)
        self.assertEqual(self.baseline.read_text(), "0\n")
        self.assertFalse(self.called.exists())

    def test_below_budget_passes_without_raw_output(self):
        self.baseline.write_text("2\n")
        result = self.run_gate("warning: synthetic warning\nsynthetic nonwarning body\n")
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertIn("current=1 baseline=2", result.stdout)
        self.assertNotIn("synthetic", result.stdout + result.stderr)

    def test_at_budget_does_not_print_raw_output(self):
        self.baseline.write_text("1\n")
        result = self.run_gate("warning: synthetic warning\nsynthetic nonwarning body\n")
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertNotIn("synthetic", result.stdout + result.stderr)

    def test_large_warning_output_is_bounded_without_pipefail_status(self):
        output = self.root / "large-cargo-output"
        output.write_text("".join(f"warning: synthetic {index}\n" for index in range(30000)))
        self.env["FAKE_CARGO_OUTPUT_FILE"] = str(output)
        result = self.run_gate()
        self.assertEqual(result.returncode, 1, result.stderr)
        self.assertIn("current=30000 baseline=0", result.stderr)
        self.assertIn("warning: synthetic 19\n", result.stderr)
        self.assertNotIn("warning: synthetic 20\n", result.stderr)
        self.assertIn("and 29980 more", result.stderr)
        self.assertLess(len(result.stderr), 2000)

    def test_zero_warning_output_is_not_echoed(self):
        result = self.run_gate("synthetic compiler progress\n")
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertNotIn("synthetic", result.stdout + result.stderr)


if __name__ == "__main__":
    unittest.main()
