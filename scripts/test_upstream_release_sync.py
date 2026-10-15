"""Local stdlib contracts for isolated release proposals, no GitHub effects."""
import os
from pathlib import Path
import re
import shutil
import subprocess
import tempfile
import textwrap
import unittest

ROOT = Path(__file__).resolve().parent.parent
WORKFLOW = (ROOT / '.github/workflows/upstream-release-sync.yml').read_text()


def run_script(name, workflow=WORKFLOW):
    """Extract a named literal run block, not whichever step happens to be last."""
    step = workflow.split(f'      - name: {name}\n', 1)[1]
    step = re.split(r'\n      - |\n  [a-z]', step, maxsplit=1)[0]
    block = step.split('        run: |\n', 1)[1]
    return textwrap.dedent(block)


class WorkflowContracts(unittest.TestCase):
    """Static checks on the workflow YAML text, without executing any steps."""
    def test_clean_job_boundary(self):
        """The validate job stays read-only and the publish job is the only one with write access."""
        validation, publication = WORKFLOW.split('\n  publish:\n')
        self.assertIn('permissions:\n  contents: read\n', validation)
        self.assertIn('    permissions:\n      contents: read\n', validation)
        self.assertNotIn(': write', validation)
        self.assertIn('persist-credentials: false', validation)
        self.assertIn('    needs: validate\n', publication)
        self.assertIn("needs.validate.outputs.needed == 'true'", publication)
        self.assertIn('    runs-on: ubuntu-latest\n', publication)
        self.assertIn('    permissions:\n      contents: write\n      pull-requests: write\n', publication)
        self.assertNotIn('uses:', publication)
        self.assertNotIn('secrets.', WORKFLOW)
        for name in ('Reconstruct pinned merge on clean runner', 'Publish review-only proposal'):
            script = run_script(name)
            self.assertNotIn('${{', script)
            self.assertNotRegex(script, r'(?m)^\s*(?:bash|sh|python3?|cargo|source|eval|npm)\s')
            self.assertNotIn('--force', script)
        reconstruct, publish = publication.split('      - name: Publish review-only proposal\n')
        self.assertNotIn('github.token', reconstruct)
        self.assertIn('GH_TOKEN: ${{ github.token }}', publish)
        self.assertIn('git init --template=', reconstruct)
        self.assertLess(reconstruct.index('git config core.hooksPath /dev/null'),
                        reconstruct.index('git fetch'))
        self.assertIn('GIT_CONFIG_GLOBAL: /dev/null', publication)
        self.assertIn('GIT_CONFIG_NOSYSTEM:', publication)
        self.assertEqual(WORKFLOW.count("vars.UPSTREAM_SYNC_ENABLED == 'true'"), 2)
        self.assertEqual(WORKFLOW.count("github.repository == 'ianalitis/jcode'"), 2)
        self.assertIn("cron: '17 */2 * * *'", WORKFLOW)
        for output in ('tag', 'tag_sha', 'upstream_sha', 'base_sha', 'tree'):
            self.assertIn('${{ needs.validate.outputs.' + output + ' }}', publication)
        self.assertIn('--state all', publish)

    def test_feature_updates_use_pr_ci_without_duplicate_push_matrix(self):
        """Keep default-branch and manual coverage without building each feature head twice."""
        ci = (ROOT / '.github/workflows/ci.yml').read_text()
        triggers = ci.split('\npermissions:', 1)[0]
        self.assertIn('  push:\n    branches: [main, master]\n', triggers)
        self.assertIn('  pull_request:\n    branches: [main, master]\n', triggers)
        self.assertIn('  workflow_dispatch:\n', triggers)

    def test_browser_session_cohort_is_bounded_and_fail_closed(self):
        """Both browser packages run under a timeout without skips or failure masking."""
        ci = (ROOT / '.github/workflows/ci.yml').read_text()
        name = 'Run browser session liveness and reuse regressions'
        self.assertEqual(ci.count(f'      - name: {name}\n'), 1)
        step = ci.split(f'      - name: {name}\n', 1)[1]
        # Explicit bash uses GitHub's -e -o pipefail defaults. Keep this step
        # unconditional, without continue-on-error or a custom permissive shell.
        self.assertEqual(step.split('        run: |\n', 1)[0],
                         '        shell: bash\n')
        script = run_script(name, workflow=ci)
        commands = [' '.join(line.split())
                    for line in script.replace('\\\n', '').splitlines() if line.strip()]
        # Exact commands also reject extra shell statements, || true, skips,
        # serialization, and unbounded or duplicate cargo invocations.
        self.assertEqual(commands, [
            'python3 .github/scripts/run_with_timeout.py 600 '
            '"$(rustup which cargo)" test --locked --target ${{ matrix.target }} '
            f'-p {package} --lib browser_session -- --nocapture'
            for package in ('jcode-base', 'jcode-app-core')
        ])

    def test_security_gate_precedes_expensive_builds(self):
        ci = (ROOT / '.github/workflows/ci.yml').read_text()
        build = ci.split('\n  build:\n', 1)[1].split('\n  windows-build-test:\n', 1)[0]
        self.assertEqual(build.count('- name: Security preflight (Linux)'), 1)
        security = build.index('- name: Security preflight (Linux)')
        self.assertLess(security, build.index('- name: Build\n'))
        gate = build[security:].split('\n      - ', 1)[0]
        self.assertIn("if: runner.os == 'Linux'", gate)
        self.assertIn('cargo install cargo-audit --locked', gate)
        self.assertIn('scripts/security_preflight.sh --strict', gate)
        self.assertNotIn('continue-on-error', gate)

    def test_manual_ci_sdk_gate_and_download_checksum(self):
        """Other workflows keep their manual trigger, repo gate, and pinned download checksum."""
        ci = (ROOT / '.github/workflows/ci.yml').read_text()
        self.assertIn('on:\n  workflow_dispatch:\n', ci)
        sdk = (ROOT / '.github/workflows/publish-typescript-sdk.yml').read_text()
        self.assertIn("    if: github.repository == '1jehuang/jcode'\n", sdk)
        self.assertLess(sdk.index("    if: github.repository == '1jehuang/jcode'"),
                        sdk.index('      id-token: write'))
        lint = (ROOT / '.github/workflows/workflow-lint.yml').read_text()
        self.assertIn('8aca8db96f1b94770f1b0d72b6dddcb1ebb8123cb3712530b08cc387b349a3d8', lint)
        self.assertLess(lint.index('sha256sum --check --strict'), lint.index('tar -xzf'))
        self.assertIn('ACTIONLINT_VERSION: "1.7.12"', lint)


class ReleaseProposalTests(unittest.TestCase):
    """Contracts for the publish step's branch/PR handling, driven by fake git and gh binaries."""
    def test_publish_and_recovery_paths(self):
        """The publish step pushes, recovers, or fails closed for each existing-branch/remote scenario."""
        script = run_script('Publish review-only proposal')
        cases = [
            ('new', {}, 0, ['push', 'pr']),
            ('recover', {'TEST_EXISTING': '1'}, 0, ['fetch', 'pr']),
            ('duplicate', {'TEST_EXISTING': '1', 'TEST_PRS': '1'}, 0, ['fetch']),
            ('divergent', {'TEST_EXISTING': '1', 'TEST_DRIFT': '1'}, 1, ['fetch']),
            ('remote-failure', {'TEST_REMOTE_FAILURE': '1'}, 7, []),
            ('stale-base', {'TEST_STALE': '1'}, 1, []),
            ('push-failure', {'TEST_PUSH_FAILURE': '1'}, 9, ['push']),
            ('pr-list-failure', {'TEST_PR_FAILURE': '1'}, 8, ['push']),
        ]
        for name, overrides, status, expected in cases:
            with self.subTest(name=name), tempfile.TemporaryDirectory(
                    dir=os.environ.get('JCODE_SCRATCH_DIR')) as directory:
                tmp = Path(directory)
                (tmp / 'upstream-proposal').mkdir()
                effects = tmp / 'effects'
                (tmp / 'git').write_text('''#!/bin/bash
set -euo pipefail
while [[ "$1" == -c ]]; do shift 2; done
case "$1" in
  ls-remote)
    if [[ "${TEST_REMOTE_FAILURE:-0}" == 1 ]]; then exit 7; fi
    if [[ "$*" == *refs/heads/master ]]; then
      if [[ "${TEST_STALE:-0}" == 1 ]]; then printf 'stale refs/heads/master\\n';
      else printf '%s refs/heads/master\\n' "$BASE_SHA"; fi
    elif [[ "${TEST_EXISTING:-0}" == 1 ]]; then printf 'abc refs/heads/proposal\\n'; fi ;;
  fetch) echo fetch >> "$TEST_EFFECTS" ;;
  rev-parse)
    if [[ "${TEST_DRIFT:-0}" == 1 ]]; then echo divergent; else echo "$TREE"; fi ;;
  push)
    echo push >> "$TEST_EFFECTS"
    if [[ "${TEST_PUSH_FAILURE:-0}" == 1 ]]; then exit 9; fi ;;
  *) exit 98 ;;
esac
''')
                (tmp / 'gh').write_text('''#!/bin/bash
set -euo pipefail
case "$1 $2" in
  'pr list')
    [[ "$*" == *'--state all'* ]]
    if [[ "${TEST_PR_FAILURE:-0}" == 1 ]]; then exit 8; fi
    printf '%s\\n' "${TEST_PRS:-0}" ;;
  'pr create') echo pr >> "$TEST_EFFECTS" ;;
  *) exit 98 ;;
esac
''')
                for executable in ('git', 'gh'):
                    (tmp / executable).chmod(0o755)
                env = dict(PATH=f'{tmp}:/usr/bin:/bin', HOME=str(tmp),
                           TAG='v1.2.3', REPO='ianalitis/jcode', BASE='master',
                           BASE_SHA='a' * 40, TREE='b' * 40,
                           RUNNER_TEMP=str(tmp), TEST_EFFECTS=str(effects))
                env.update(overrides)
                result = subprocess.run(['bash', '-c', script], env=env,
                                        capture_output=True, text=True, timeout=10)
                self.assertEqual(result.returncode, status, result.stderr)
                observed = effects.read_text().splitlines() if effects.exists() else []
                self.assertEqual(observed, expected)


class NativeGitContracts(unittest.TestCase):
    """Real merges in isolated scratch repos. Only file:// transport is allowed."""
    def setUp(self):
        """Build a fork/upstream/validation git fixture and fake `gh` binary for the native tests."""
        self.scratch = tempfile.TemporaryDirectory(dir=os.environ.get('JCODE_SCRATCH_DIR'))
        self.addCleanup(self.scratch.cleanup)
        self.tmp = Path(self.scratch.name)
        self.git_bin = shutil.which('git')
        self.env = dict(PATH='/usr/bin:/bin', HOME=str(self.tmp),
                        GIT_CONFIG_NOSYSTEM='1', GIT_CONFIG_GLOBAL='/dev/null',
                        GIT_ALLOW_PROTOCOL='file', GIT_TERMINAL_PROMPT='0',
                        GIT_AUTHOR_NAME='Fixture', GIT_AUTHOR_EMAIL='fixture@example.invalid',
                        GIT_COMMITTER_NAME='Fixture', GIT_COMMITTER_EMAIL='fixture@example.invalid',
                        GIT_CONFIG_COUNT='2',
                        GIT_CONFIG_KEY_0=f'url.{self.tmp / "fork"}.insteadOf',
                        GIT_CONFIG_VALUE_0='https://github.com/ianalitis/jcode.git',
                        GIT_CONFIG_KEY_1=f'url.{self.tmp / "upstream"}.insteadOf',
                        GIT_CONFIG_VALUE_1='https://github.com/1jehuang/jcode.git',
                        REPO='ianalitis/jcode', BASE='master', RUNNER_TEMP=str(self.tmp),
                        GITHUB_OUTPUT=str(self.tmp / 'outputs'))
        self.fork = self.tmp / 'fork'
        self.git(self.tmp, 'init', '--template=', '--initial-branch=master', str(self.fork))
        self.commit(self.fork, 'common', 'common\n')
        self.upstream = self.tmp / 'upstream'
        self.git(self.tmp, 'clone', '--template=', str(self.fork), str(self.upstream))
        self.commit(self.fork, 'fork-only', 'keep fork changes\n')
        self.commit(self.upstream, 'upstream-only', 'upstream change\n')
        self.commit(self.upstream, 'untrusted.sh', 'touch "$RUNNER_TEMP/imported-executed"\n')
        self.git(self.upstream, 'tag', '-a', 'v1.2.3', '-m', 'Published fixture')
        self.validation = self.tmp / 'validation'
        self.git(self.tmp, 'clone', '--template=', str(self.fork), str(self.validation))
        bin_dir = self.tmp / 'bin'
        bin_dir.mkdir()
        (bin_dir / 'git').symlink_to(self.git_bin)
        (bin_dir / 'gh').write_text('''#!/bin/bash
set -eu
case "$1 $2" in
  'release view') printf '%s\\n' "${TEST_TAG:-v1.2.3}" ;;
  'pr list') echo 0 ;;
  'pr create') echo created > "$RUNNER_TEMP/pr-created" ;;
  *) exit 98 ;;
esac
''')
        (bin_dir / 'gh').chmod(0o755)
        self.env['PATH'] = f'{bin_dir}:/usr/bin:/bin'

    def git(self, cwd, *args):
        """Run a real git command in `cwd` with the fixture's isolated environment and return stdout."""
        return subprocess.run([self.git_bin, *args], cwd=cwd, env=self.env,
                              capture_output=True, text=True, check=True, timeout=15).stdout.strip()

    def commit(self, repo, path, content):
        """Write `content` to `path` in `repo` and commit it."""
        (repo / path).write_text(content)
        self.git(repo, 'add', path)
        self.git(repo, 'commit', '-m', path)

    def shell(self, step, **overrides):
        """Run a named workflow step's script in the validation checkout with optional env overrides."""
        return subprocess.run(['bash', '-c', run_script(step)], cwd=self.validation,
                              env=dict(self.env, **overrides), capture_output=True,
                              text=True, timeout=20)

    def pin(self):
        """Run the pin-and-merge step, assert it succeeded and found work, and apply its outputs to self.env."""
        result = self.shell('Pin and merge latest published stable release')
        self.assertEqual(result.returncode, 0, result.stderr)
        outputs = dict(line.split('=', 1) for line in (self.tmp / 'outputs').read_text().splitlines())
        self.assertEqual(outputs.pop('needed'), 'true')
        self.env.update({key.upper(): value for key, value in outputs.items()})
        return outputs

    def test_exact_merge_and_no_imported_execution(self):
        """The clean reconstruction and publish steps reproduce the pinned merge without running imported hooks."""
        self.pin()
        # Model untrusted validation installing a credential-stealing pre-push
        # hook. The clean publisher must neither inherit nor invoke it.
        hooks = self.validation / '.git/hooks'
        hooks.mkdir()
        hook = hooks / 'pre-push'
        hook.write_text('#!/bin/sh\ntouch "$RUNNER_TEMP/hook-executed"\nexit 1\n')
        hook.chmod(0o755)
        self.git(self.validation, 'config', 'core.hooksPath', str(hooks))
        self.assertNotEqual(self.env['TAG_SHA'], self.env['UPSTREAM_SHA'])
        result = self.shell('Reconstruct pinned merge on clean runner')
        self.assertEqual(result.returncode, 0, result.stderr)
        proposal = self.tmp / 'upstream-proposal'
        self.assertEqual(self.git(proposal, 'rev-parse', 'HEAD^{tree}'), self.env['TREE'])
        self.assertEqual(self.git(proposal, 'config', 'core.hooksPath'), '/dev/null')
        self.assertEqual((proposal / 'fork-only').read_text(), 'keep fork changes\n')
        self.assertFalse((self.tmp / 'imported-executed').exists())
        self.assertNotIn('credential', (proposal / '.git/config').read_text())
        result = self.shell('Publish review-only proposal')
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(self.git(self.fork, 'rev-parse',
                                  'refs/heads/automation/upstream-v1.2.3^{tree}'),
                         self.env['TREE'])
        self.assertTrue((self.tmp / 'pr-created').exists())
        self.assertFalse((self.tmp / 'hook-executed').exists())
        self.assertFalse((self.tmp / 'imported-executed').exists())

    def test_invalid_inputs_fail_before_git_init(self):
        """Malicious or malformed step inputs are rejected before any git repository is created."""
        self.pin()
        for key, bad in [('TAG', 'v1.2.3\nforged=true'), ('TAG', '$(touch injected)'),
                         ('BASE', '../invalid'), ('REPO', 'other/repo')] + [
                             (key, '$(touch injected)') for key in
                             ('BASE_SHA', 'TAG_SHA', 'UPSTREAM_SHA', 'TREE')]:
            with self.subTest(key=key, bad=bad):
                result = self.shell('Reconstruct pinned merge on clean runner', **{key: bad})
                self.assertNotEqual(result.returncode, 0)
                self.assertFalse((self.tmp / 'upstream-proposal').exists())
                self.assertFalse((self.validation / 'injected').exists())

    def test_pinned_hash_mismatches_fail_closed(self):
        """A mismatch against any pinned hash aborts the reconstruction step."""
        self.pin()
        for key in ('BASE_SHA', 'TAG_SHA', 'UPSTREAM_SHA', 'TREE'):
            with self.subTest(key=key):
                runner = self.tmp / key
                runner.mkdir()
                result = self.shell('Reconstruct pinned merge on clean runner',
                                    RUNNER_TEMP=str(runner), **{key: '0' * 40})
                self.assertNotEqual(result.returncode, 0)
                if key == 'TREE':
                    self.assertIn('Reconstructed tree differs', result.stderr)

    def test_remote_base_advance_fails_closed(self):
        """If the fork's default branch advances after pinning, reconstruction fails closed."""
        self.pin()
        self.commit(self.fork, 'later', 'advanced\n')
        result = self.shell('Reconstruct pinned merge on clean runner')
        self.assertNotEqual(result.returncode, 0)
        self.assertIn('Default branch advanced', result.stderr)

    def test_moved_tag_fails_closed(self):
        """If the upstream release tag is force-moved after pinning, reconstruction fails closed."""
        self.pin()
        self.commit(self.upstream, 'later', 'retagged\n')
        self.git(self.upstream, 'tag', '-f', '-a', 'v1.2.3', '-m', 'Moved fixture')
        result = self.shell('Reconstruct pinned merge on clean runner')
        self.assertNotEqual(result.returncode, 0)

    def test_ancestor_is_noop(self):
        """An already-merged ancestor release requires no sync and produces `needed=false`."""
        self.git(self.upstream, 'tag', 'v1.0.0', 'HEAD~2')
        result = self.shell('Pin and merge latest published stable release', TEST_TAG='v1.0.0')
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual((self.tmp / 'outputs').read_text(), 'needed=false\n')

    def test_conflict_is_not_published(self):
        """A merge conflict blocks the sync, leaves no outputs, and surfaces a summary for review."""
        self.commit(self.validation, 'common', 'fork conflict\n')
        self.commit(self.upstream, 'common', 'upstream conflict\n')
        self.git(self.upstream, 'tag', 'v1.2.4')
        summary = self.tmp / 'summary'
        result = self.shell('Pin and merge latest published stable release', TEST_TAG='v1.2.4',
                            GITHUB_STEP_SUMMARY=str(summary))
        self.assertNotEqual(result.returncode, 0)
        self.assertFalse((self.tmp / 'outputs').exists())
        text = summary.read_text()
        self.assertIn('Upstream release sync blocked', text)
        self.assertIn('v1.2.4', text)
        self.assertIn('"common"', text)
        self.assertIn('No proposal was published', text)
        self.assertIn(self.git(self.validation, 'rev-parse', 'HEAD'), text)


if __name__ == '__main__':
    unittest.main()
