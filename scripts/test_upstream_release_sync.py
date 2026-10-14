"""Local-only contracts for release proposal recovery, no GitHub effects."""
import os
from pathlib import Path
import subprocess
import tempfile
import textwrap
import unittest

ROOT = Path(__file__).resolve().parent.parent


class ReleaseProposalTests(unittest.TestCase):
    def test_publish_and_recovery_paths(self):
        workflow = (ROOT / '.github/workflows/upstream-release-sync.yml').read_text()
        script = textwrap.dedent(workflow.rsplit('        run: |\n', 1)[1])
        cases = [
            ('new', {}, 0, ['push', 'pr']),
            ('recover', {'TEST_EXISTING': '1'}, 0, ['fetch', 'pr']),
            ('duplicate', {'TEST_EXISTING': '1', 'TEST_PRS': '1'}, 0, ['fetch']),
            ('divergent', {'TEST_EXISTING': '1', 'TEST_DRIFT': '1'}, 1, ['fetch']),
            ('remote-failure', {'TEST_REMOTE_FAILURE': '1'}, 7, []),
        ]
        for name, overrides, status, expected in cases:
            with self.subTest(name=name), tempfile.TemporaryDirectory() as directory:
                tmp = Path(directory)
                effects = tmp / 'effects'
                (tmp / 'git').write_text('''#!/bin/bash
set -euo pipefail
case "$1" in
  ls-remote)
    if [[ "${TEST_REMOTE_FAILURE:-0}" == 1 ]]; then exit 7; fi
    if [[ "${TEST_EXISTING:-0}" == 1 ]]; then printf 'abc refs/heads/proposal\\n'; fi ;;
  fetch) echo fetch >> "$TEST_EFFECTS" ;;
  diff) exit "${TEST_DRIFT:-0}" ;;
  push) echo push >> "$TEST_EFFECTS" ;;
  *) exit 98 ;;
esac
''')
                (tmp / 'gh').write_text('''#!/bin/bash
set -euo pipefail
case "$1 $2" in
  'auth setup-git') : ;;
  'pr list') printf '%s\\n' "${TEST_PRS:-0}" ;;
  'pr create') echo pr >> "$TEST_EFFECTS" ;;
  *) exit 98 ;;
esac
''')
                for executable in ('git', 'gh'):
                    (tmp / executable).chmod(0o755)
                env = dict(os.environ, PATH=f'{tmp}:{os.environ["PATH"]}',
                           TAG='v1.2.3', REPO='test/fork', BASE='master',
                           TEST_EFFECTS=str(effects))
                for key in ('TEST_EXISTING', 'TEST_DRIFT', 'TEST_PRS', 'TEST_REMOTE_FAILURE'):
                    env.pop(key, None)
                env.update(overrides)
                result = subprocess.run(['bash', '-c', script], env=env,
                                        capture_output=True, text=True, timeout=10)
                self.assertEqual(result.returncode, status, result.stderr)
                observed = effects.read_text().splitlines() if effects.exists() else []
                self.assertEqual(observed, expected)


if __name__ == '__main__':
    unittest.main()
