"""Offline contracts for the read-only fork health command."""
import subprocess
import unittest

from fork_health import report


class HealthTests(unittest.TestCase):
    def fetch(self, endpoint):
        self.calls.append(endpoint)
        if endpoint == 'repos/owner/fork':
            return dict(fork=True, default_branch='release/custom',
                        parent=dict(full_name='upstream/project', default_branch='main'))
        if endpoint.endswith('/releases/latest'):
            if self.release_error:
                raise self.release_error
            return dict(tag_name='v1.2.3')
        return dict(status='diverged', ahead_by=8, behind_by=3)

    def setUp(self):
        self.calls = []
        self.release_error = None

    def test_separate_default_and_release_distance(self):
        result = report('owner/fork', self.fetch)
        self.assertEqual(result['ahead'], 8)
        self.assertEqual(result['behind'], 3)
        self.assertEqual(result['release_behind'], 3)
        self.assertEqual(self.calls, [
            'repos/owner/fork',
            'repos/upstream/project/compare/main...owner%3Arelease%2Fcustom',
            'repos/upstream/project/releases/latest',
            'repos/upstream/project/compare/v1.2.3...owner%3Arelease%2Fcustom',
        ])

    def test_no_release_is_unknown_not_current(self):
        self.release_error = subprocess.CalledProcessError(1, 'gh', stderr='gh: Not Found (HTTP 404)')
        result = report('owner/fork', self.fetch)
        self.assertIsNone(result['release_behind'])
        self.assertIsNone(result['latest_release'])

    def test_api_errors_are_not_healthy(self):
        for status in (401, 403, 429, 500):
            with self.subTest(status=status):
                self.release_error = subprocess.CalledProcessError(1, 'gh', stderr=f'(HTTP {status})')
                with self.assertRaises(subprocess.CalledProcessError):
                    report('owner/fork', self.fetch)

    def test_reject_nonfork_and_invalid_names(self):
        with self.assertRaises(ValueError):
            report('owner/fork', lambda _: dict(fork=False))
        for name in ('--help', 'owner/repo/extra', 'owner/repo?x=1', 'owner/repo\n'):
            with self.subTest(name=name), self.assertRaises(ValueError):
                report(name, self.fetch)
        self.assertEqual(self.calls, [])


if __name__ == '__main__':
    unittest.main()
