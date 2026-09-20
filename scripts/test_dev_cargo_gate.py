#!/usr/bin/env python3
"""Fake-Cargo concurrency regressions for scripts/dev_cargo.sh."""

import json
import os
import shutil
import signal
import subprocess
import sys
import tempfile
import time
import unittest
from pathlib import Path

REPO = Path(__file__).resolve().parents[1]
WRAPPER = REPO / "scripts/dev_cargo.sh"


class TestDevCargoGate(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory(
            prefix="dev cargo gate with spaces-",
            dir=os.environ.get("JCODE_SCRATCH_DIR"),
        )
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.bin = self.root / "bin without flock"
        self.home = self.root / "home"
        self.work = self.root / "work"
        self.state = self.root / "fake cargo state"
        self.gate_dir = self.root / "gate dir with spaces"
        for path in (self.bin, self.home, self.work, self.state, self.gate_dir):
            path.mkdir()
        self.gate_path = self.gate_dir / "jcode cargo build.lock"
        self.actions = self.root / "rust actions.jsonl"
        self.events = self.state / "events.log"
        self.overlap = self.state / "overlap"
        self.active = self.state / "active"
        self.release = self.state / "release"

        for command in ("date", "dirname", "mkdir"):
            target = shutil.which(command)
            if target is None:
                self.skipTest(f"required fixture command is unavailable: {command}")
            os.symlink(target, self.bin / command)
        os.symlink(sys.executable, self.bin / "python3")
        uname = self.bin / "uname"
        uname.write_text(
            "#!/bin/bash\n"
            "case \"${1:-}\" in\n"
            "  -s) printf 'Darwin\\n' ;;\n"
            "  -m) printf 'arm64\\n' ;;\n"
            "  *) printf 'Darwin\\n' ;;\n"
            "esac\n"
        )
        uname.chmod(0o700)

        cargo = self.bin / "cargo"
        cargo.write_text(
            '''#!/usr/bin/env python3
import json, os
from pathlib import Path
import subprocess, sys, time

state = Path(os.environ["TEST_GATE_STATE"])
events = state / "events.log"
active = state / "active"
overlap = state / "overlap"
depth = int(os.environ.get("TEST_NESTED_DEPTH", "0"))

def event(name):
    row = {
        "event": name,
        "pid": os.getpid(),
        "depth": depth,
        "gate_held": os.environ.get("JCODE_CARGO_GATE_HELD"),
        "time_ns": time.time_ns(),
    }
    fd = os.open(events, os.O_WRONLY | os.O_CREAT | os.O_APPEND, 0o600)
    try:
        os.write(fd, (json.dumps(row) + "\\n").encode())
    finally:
        os.close(fd)

if os.environ.get("TEST_NESTED") == "1" and depth == 0:
    event("outer-start")
    env = os.environ.copy()
    env["TEST_NESTED_DEPTH"] = "1"
    result = subprocess.run(["/bin/bash", env["TEST_WRAPPER"], "check"], env=env)
    event("outer-end")
    sys.exit(result.returncode)

owned_active = False
try:
    active.mkdir()
    owned_active = True
except FileExistsError:
    overlap.write_text("overlap")
event("start")
pid_file = os.environ.get("TEST_CARGO_PID_FILE")
if pid_file:
    Path(pid_file).write_text(str(os.getpid()))
release_file = os.environ.get("TEST_CARGO_RELEASE_FILE")
if release_file:
    release = Path(release_file)
    deadline = time.monotonic() + 10
    while not release.exists() and time.monotonic() < deadline:
        time.sleep(0.02)
else:
    time.sleep(float(os.environ.get("TEST_CARGO_SLEEP", "0.15")))
event("end")
if owned_active:
    active.rmdir()
sys.exit(int(os.environ.get("TEST_CARGO_STATUS", "0")))
'''
        )
        cargo.chmod(0o700)

        self.env = {
            "PATH": str(self.bin),
            "HOME": str(self.home),
            "TMPDIR": str(self.work),
            "JCODE_REMOTE_CONFIG": "/dev/null",
            "JCODE_REMOTE_CARGO": "0",
            "JCODE_TEST_STATE_ISOLATION": "off",
            "JCODE_BUILD_GIT_HASH": "test",
            "JCODE_BUILD_JOBS": "1",
            "JCODE_PARALLEL_FRONTEND": "0",
            "JCODE_FAST_LINKER": "system",
            "JCODE_CARGO_GATE": "on",
            "JCODE_CARGO_GATE_PATH": str(self.gate_path),
            "JCODE_RUST_ACTION_LOG_PATH": str(self.actions),
            "SCCACHE_DISABLE": "1",
            "TEST_GATE_STATE": str(self.state),
            "TEST_WRAPPER": str(WRAPPER),
        }

    def popen(self, **overrides):
        return subprocess.Popen(
            ["/bin/bash", str(WRAPPER), "check"],
            cwd=REPO,
            env={**self.env, **overrides},
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            text=True,
        )

    def run_wrapper(self, **overrides):
        return subprocess.run(
            ["/bin/bash", str(WRAPPER), "check"],
            cwd=REPO,
            env={**self.env, **overrides},
            capture_output=True,
            text=True,
            timeout=10,
            check=False,
        )

    def read_events(self):
        if not self.events.exists():
            return []
        return [json.loads(line) for line in self.events.read_text().splitlines()]

    def wait_for_events(self, count, timeout=3):
        deadline = time.monotonic() + timeout
        while time.monotonic() < deadline:
            events = self.read_events()
            if len(events) >= count:
                return events
            time.sleep(0.01)
        self.fail(f"timed out waiting for {count} fake-Cargo events: {self.read_events()}")

    def test_missing_flock_fallback_serializes_and_records_wait(self):
        first = self.popen(TEST_CARGO_SLEEP="0.4")
        self.wait_for_events(1)
        second = self.popen(TEST_CARGO_SLEEP="0.05")
        first_stdout, first_stderr = first.communicate(timeout=5)
        second_stdout, second_stderr = second.communicate(timeout=5)
        self.assertEqual(first.returncode, 0, first_stderr + first_stdout)
        self.assertEqual(second.returncode, 0, second_stderr + second_stdout)
        self.assertFalse(self.overlap.exists(), self.read_events())
        self.assertEqual(
            [row["event"] for row in self.read_events()],
            ["start", "end", "start", "end"],
        )
        self.assertNotIn("flock is unavailable", first_stderr + second_stderr)
        self.assertIn("waiting for the host-wide Cargo gate", second_stderr)
        self.assertIn("acquired host-wide Cargo gate", second_stderr)
        rows = [json.loads(line) for line in self.actions.read_text().splitlines()]
        waits = sorted(row["gate_wait_ms"] for row in rows)
        self.assertEqual(waits[0], 0)
        self.assertGreater(waits[1], 0)

    def test_nested_wrapper_inherits_gate_without_deadlock(self):
        result = self.run_wrapper(TEST_NESTED="1")
        self.assertEqual(result.returncode, 0, result.stderr)
        events = self.read_events()
        self.assertEqual(
            [(row["event"], row["depth"]) for row in events],
            [("outer-start", 0), ("start", 1), ("end", 1), ("outer-end", 0)],
        )
        self.assertTrue(all(row["gate_held"] == "1" for row in events))

    def test_nonzero_exit_releases_gate(self):
        failed = self.run_wrapper(TEST_CARGO_STATUS="23")
        self.assertEqual(failed.returncode, 23, failed.stderr)
        succeeded = self.run_wrapper()
        self.assertEqual(succeeded.returncode, 0, succeeded.stderr)
        self.assertFalse(self.overlap.exists(), self.read_events())

    def test_child_keeps_gate_after_wrapper_is_killed(self):
        child_pid_file = self.state / "cargo.pid"
        first = self.popen(
            TEST_CARGO_RELEASE_FILE=str(self.release),
            TEST_CARGO_PID_FILE=str(child_pid_file),
        )
        self.wait_for_events(1)
        os.kill(first.pid, signal.SIGKILL)
        first.wait(timeout=2)
        if first.stdout is not None:
            first.stdout.close()
        if first.stderr is not None:
            first.stderr.close()
        self.assertTrue(child_pid_file.exists())

        contender = self.popen(TEST_CARGO_SLEEP="0.05")
        time.sleep(0.2)
        self.assertIsNone(contender.poll(), "contender passed gate while first Cargo survived")
        self.assertEqual(len(self.read_events()), 1)
        self.assertFalse(self.overlap.exists())

        self.release.write_text("release")
        contender_stdout, contender_stderr = contender.communicate(timeout=5)
        self.assertEqual(contender.returncode, 0, contender_stderr + contender_stdout)
        self.assertFalse(self.overlap.exists(), self.read_events())
        self.assertEqual(
            [row["event"] for row in self.read_events()],
            ["start", "end", "start", "end"],
        )

    def test_explicit_gate_off_still_allows_concurrency(self):
        first = self.popen(JCODE_CARGO_GATE="off", TEST_CARGO_SLEEP="0.35")
        self.wait_for_events(1)
        second = self.popen(JCODE_CARGO_GATE="off", TEST_CARGO_SLEEP="0.05")
        first.communicate(timeout=5)
        second.communicate(timeout=5)
        self.assertEqual(first.returncode, 0)
        self.assertEqual(second.returncode, 0)
        self.assertTrue(self.overlap.exists(), self.read_events())

    @unittest.skipUnless(shutil.which("flock"), "native flock executable unavailable")
    def test_fallback_interoperates_with_native_flock(self):
        native_flock = shutil.which("flock")
        ready = self.state / "native-ready"
        holder = subprocess.Popen(
            [
                native_flock,
                "-x",
                str(self.gate_path),
                sys.executable,
                "-c",
                (
                    "from pathlib import Path; import time; "
                    f"Path({str(ready)!r}).write_text('ready'); time.sleep(0.4)"
                ),
            ]
        )
        deadline = time.monotonic() + 2
        while not ready.exists() and time.monotonic() < deadline:
            time.sleep(0.01)
        self.assertTrue(ready.exists(), "native flock holder did not start")
        fallback = self.popen(TEST_CARGO_SLEEP="0.05")
        time.sleep(0.15)
        self.assertEqual(self.read_events(), [])
        holder.wait(timeout=2)
        _, fallback_stderr = fallback.communicate(timeout=5)
        self.assertEqual(fallback.returncode, 0, fallback_stderr)
        self.assertIn("waiting for the host-wide Cargo gate", fallback_stderr)

        self.events.unlink()
        self.release.unlink(missing_ok=True)
        fallback_holder = self.popen(TEST_CARGO_RELEASE_FILE=str(self.release))
        self.wait_for_events(1)
        blocked = subprocess.run(
            [native_flock, "-n", str(self.gate_path), "/usr/bin/true"],
            capture_output=True,
            check=False,
        )
        self.assertNotEqual(blocked.returncode, 0)
        self.release.write_text("release")
        fallback_holder.communicate(timeout=5)
        acquired = subprocess.run(
            [native_flock, "-n", str(self.gate_path), "/usr/bin/true"],
            capture_output=True,
            check=False,
        )
        self.assertEqual(acquired.returncode, 0, acquired.stderr)


if __name__ == "__main__":
    unittest.main(verbosity=2)
