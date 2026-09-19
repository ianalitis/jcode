#!/usr/bin/env python3
"""Exercise the real Cargo wrapper with a state-writing child, never live state."""

import json
import os
from pathlib import Path
import subprocess
import tempfile
import unittest


REPO = Path(__file__).resolve().parents[1]
WRAPPER = REPO / "scripts/dev_cargo.sh"


class TestLocalTestState(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory(
            prefix="cargo-isolation-", dir=os.environ.get("JCODE_SCRATCH_DIR")
        )
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        for name in (
            "bin",
            "home",
            "work",
            "scratch",
            "original-home",
            "original-runtime",
        ):
            (self.root / name).mkdir()
        self.home = self.root / "original-home"
        self.runtime = self.root / "original-runtime"
        for path in (self.home, self.runtime):
            (path / "sentinel").write_text("preserve me")
        cargo = self.root / "bin/cargo"
        cargo.write_text('''#!/usr/bin/env python3
import json, os, pathlib, signal, sys, time
paths = [pathlib.Path(os.environ[k]) for k in ("JCODE_HOME", "JCODE_RUNTIME_DIR")]
record = {"paths": [str(p) for p in paths],
          "modes": [p.stat().st_mode & 0o777 for p in paths],
          "parent_mode": paths[0].parent.stat().st_mode & 0o777}
pathlib.Path(os.environ["TEST_CHILD_RECORD"]).write_text(json.dumps(record))
for path in paths:
    (path / "child-write").write_text("test state")
if os.environ.get("TEST_CARGO_SIGNAL") == "TERM":
    os.kill(os.getppid(), signal.SIGTERM)
    time.sleep(0.1)
print("running " + os.environ.get("TEST_COUNT", "1") + " tests")
sys.exit(int(os.environ.get("TEST_CARGO_STATUS", "0")))
''')
        cargo.chmod(0o700)
        ssh = self.root / "bin/ssh"
        ssh.write_text('''#!/usr/bin/env python3
import json, os, pathlib, sys
path = pathlib.Path(os.environ["TEST_REMOTE_RECORD"])
with path.open("a") as stream:
    stream.write(json.dumps({
        "argv": sys.argv[1:],
        "jcode_home": os.environ.get("JCODE_HOME"),
        "runtime": os.environ.get("JCODE_RUNTIME_DIR"),
    }) + "\\n")
if "printf 'jcode-remote-ok\\n'" in sys.argv:
    print("jcode-remote-ok")
''')
        ssh.chmod(0o700)
        rsync = self.root / "bin/rsync"
        rsync.write_text("#!/usr/bin/env bash\nexit 0\n")
        rsync.chmod(0o700)
        self.env = {
            "PATH": str(self.root / "bin") + os.pathsep + os.environ["PATH"],
            "HOME": str(self.root / "home"),
            "TMPDIR": str(self.root / "work"),
            "JCODE_HOME": str(self.home),
            "JCODE_RUNTIME_DIR": str(self.runtime),
            "JCODE_REMOTE_CONFIG": "/dev/null",
            "JCODE_REMOTE_CARGO": "0",
            "JCODE_BUILD_GIT_HASH": "test",
            "JCODE_BUILD_JOBS": "1",
            "JCODE_PARALLEL_FRONTEND": "0",
            "SCCACHE_DISABLE": "1",
            "JCODE_FAST_LINKER": "system",
            "JCODE_CARGO_GATE": "off",
            "JCODE_RUST_ACTION_LOG_PATH": str(self.root / "actions.jsonl"),
            "TEST_CHILD_RECORD": str(self.root / "child.json"),
            "TEST_REMOTE_RECORD": str(self.root / "remote.jsonl"),
        }

    def run_wrapper(self, *args, **overrides):
        return subprocess.run(
            ["bash", str(WRAPPER), *args],
            cwd=REPO,
            env={**self.env, **overrides},
            capture_output=True,
            text=True,
            timeout=10,
        )

    def assert_isolated_and_removed(self, expected_parent=None):
        record = json.loads((self.root / "child.json").read_text())
        home, runtime = map(Path, record["paths"])
        self.assertNotEqual(home, self.home, "test child inherited operator JCODE_HOME")
        self.assertNotEqual(runtime, self.runtime, "test child inherited operator runtime")
        self.assertEqual(home.parent, runtime.parent)
        self.assertEqual(home.parent.parent, expected_parent or self.root / "work")
        self.assertEqual(record["modes"], [0o700, 0o700])
        self.assertEqual(record["parent_mode"], 0o700)
        self.assertFalse(home.parent.exists(), "owned test state leaked after exit")
        for original in (self.home, self.runtime):
            self.assertEqual((original / "sentinel").read_text(), "preserve me")
            self.assertFalse((original / "child-write").exists())
        return str(home.parent)

    def test_default_success_failure_and_logging_off(self):
        for status in (0, 23):
            for logging in ("on", "off"):
                with self.subTest(status=status, logging=logging):
                    result = self.run_wrapper(
                        "test", TEST_CARGO_STATUS=str(status), JCODE_RUST_ACTION_LOG=logging
                    )
                    self.assertEqual(result.returncode, status, result.stderr)
                    self.assert_isolated_and_removed()
        rows = [
            json.loads(line)
            for line in (self.root / "actions.jsonl").read_text().splitlines()
        ]
        self.assertEqual([row["exit_code"] for row in rows], [0, 23])

    def test_toolchain_prefix_and_alias(self):
        for args in (("+stable", "test"), ("t",), ("+stable", "t")):
            with self.subTest(args=args):
                result = self.run_wrapper(*args)
                self.assertEqual(result.returncode, 0, result.stderr)
                self.assert_isolated_and_removed()

    def test_explicit_opt_out_preserves_inherited_paths(self):
        result = self.run_wrapper("test", JCODE_TEST_STATE_ISOLATION="off")
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertIn("isolation disabled", result.stderr)
        record = json.loads((self.root / "child.json").read_text())
        self.assertEqual(record["paths"], [str(self.home), str(self.runtime)])
        self.assertTrue((self.home / "child-write").exists())

    def test_non_test_command_is_unchanged(self):
        result = self.run_wrapper("metadata")
        self.assertEqual(result.returncode, 0, result.stderr)
        record = json.loads((self.root / "child.json").read_text())
        self.assertEqual(record["paths"], [str(self.home), str(self.runtime)])
        self.assertFalse(list((self.root / "work").iterdir()))

    def test_existing_scratch_directory_is_preferred(self):
        result = self.run_wrapper("test", JCODE_SCRATCH_DIR=str(self.root / "scratch"))
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assert_isolated_and_removed(self.root / "scratch")

    def test_each_invocation_gets_fresh_state(self):
        roots = []
        for _ in range(2):
            result = self.run_wrapper("test")
            self.assertEqual(result.returncode, 0, result.stderr)
            roots.append(self.assert_isolated_and_removed())
        self.assertNotEqual(*roots)

    def test_setup_failure_never_runs_cargo(self):
        result = self.run_wrapper("test", TMPDIR=str(self.root / "missing/dir"))
        self.assertNotEqual(result.returncode, 0)
        self.assertFalse((self.root / "child.json").exists())
        self.assertEqual((self.home / "sentinel").read_text(), "preserve me")

    def test_signal_exit_cleans_owned_state(self):
        result = self.run_wrapper(
            "test", TEST_CARGO_SIGNAL="TERM", JCODE_RUST_ACTION_LOG="off"
        )
        self.assertNotEqual(result.returncode, 0)
        self.assert_isolated_and_removed()

    def test_zero_match_failure_still_cleans_up(self):
        result = self.run_wrapper("test", "missing_test", TEST_COUNT="0")
        self.assertEqual(result.returncode, 97, result.stderr)
        self.assert_isolated_and_removed()
        record = json.loads((self.root / "actions.jsonl").read_text())
        self.assertEqual(record["exit_code"], 97)

    def test_default_log_sink_survives_test_cleanup(self):
        del self.env["JCODE_RUST_ACTION_LOG_PATH"]
        result = self.run_wrapper("test")
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assert_isolated_and_removed()
        record = json.loads((self.home / "logs/rust-actions.jsonl").read_text())
        self.assertEqual(record["exit_code"], 0)

    def test_remote_test_keeps_inherited_state(self):
        result = self.run_wrapper(
            "test",
            JCODE_REMOTE_CARGO="1",
            JCODE_REMOTE_HOST="synthetic-host",
            JCODE_REMOTE_SSH_BIN=str(self.root / "bin/ssh"),
            JCODE_REMOTE_RSYNC_BIN=str(self.root / "bin/rsync"),
            JCODE_REMOTE_TCP_PROBE="0",
        )
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertFalse((self.root / "child.json").exists(), "local Cargo unexpectedly ran")
        records = [
            json.loads(line)
            for line in (self.root / "remote.jsonl").read_text().splitlines()
        ]
        self.assertTrue(records)
        for record in records:
            self.assertEqual(record["jcode_home"], str(self.home))
            self.assertEqual(record["runtime"], str(self.runtime))
            self.assertNotIn("jcode-test-state.", json.dumps(record))
        self.assertFalse(list((self.root / "work").iterdir()))


if __name__ == "__main__":
    unittest.main(verbosity=2)
