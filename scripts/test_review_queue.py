#!/usr/bin/env python3
"""Offline contract check for the read-only portfolio queue."""
import json
import os
from pathlib import Path
import subprocess
import tempfile
import unittest


class ReviewQueueTest(unittest.TestCase):
    def test_only_reads_allowlisted_repositories_and_propagates_failure(self):
        script = Path(__file__).with_name("review_queue.sh")
        repos = ["1jehuang/jcode", "1jehuang/handterm",
                 "1jehuang/mermaid-rs-renderer", "1jehuang/agentgrep"]
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            fake = root / "gh"
            fake.write_text("""#!/usr/bin/env python3
import json, os, sys
with open(os.environ['QUEUE_CALLS'], 'a') as log:
    log.write(json.dumps(sys.argv[1:]) + '\\n')
if os.environ.get('QUEUE_FAIL'):
    sys.exit(7)
print('{}')
""")
            fake.chmod(0o755)
            calls = root / "calls"
            env = dict(os.environ, PATH=f"{root}:{os.environ['PATH']}",
                       QUEUE_CALLS=str(calls))
            result = subprocess.run(["bash", str(script)], env=env,
                                    capture_output=True, text=True, timeout=10)
            self.assertEqual(result.returncode, 0, result.stderr)
            commands = [json.loads(line) for line in calls.read_text().splitlines()]
            self.assertEqual(len(commands), 4)
            for command, repo in zip(commands, repos):
                self.assertEqual(command[:2], ["pr", "list"])
                self.assertEqual(command[command.index("--repo") + 1], repo)
                self.assertEqual(command[command.index("--state") + 1], "open")
                self.assertEqual(command[command.index("--author") + 1], "ianalitis")
                self.assertEqual(command[command.index("--limit") + 1], "100")
                self.assertIn("--json", command)
            failed = subprocess.run(["bash", str(script)],
                                    env=dict(env, QUEUE_FAIL="1"),
                                    capture_output=True, timeout=10)
            self.assertEqual(failed.returncode, 7)
            self.assertEqual(len(calls.read_text().splitlines()), 5)


if __name__ == "__main__":
    unittest.main()
