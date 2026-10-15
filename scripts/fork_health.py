"""Read-only GitHub fork health report. Requires an authenticated gh CLI."""
import argparse
import json
import re
import subprocess
from urllib.parse import quote


REPOSITORY = re.compile(r"[A-Za-z0-9_.-]+/[A-Za-z0-9_.-]+")


def api(endpoint):
    """Call the GitHub API via `gh api` and parse the JSON response."""
    result = subprocess.run(
        ['gh', 'api', endpoint], check=True, capture_output=True, text=True, timeout=60
    )
    return json.loads(result.stdout)


def report(repository, fetch=api):
    """Build a fork health report comparing a fork against its upstream parent and latest release."""
    if not REPOSITORY.fullmatch(repository):
        raise ValueError('Expected owner/repository')
    metadata = fetch(f'repos/{repository}')
    if not metadata.get('fork') or not metadata.get('parent'):
        raise ValueError(f'{repository} is not a fork with an accessible parent')
    parent = metadata['parent']
    upstream = parent['full_name']
    if not REPOSITORY.fullmatch(upstream):
        raise ValueError('Invalid parent repository')
    base = quote(parent['default_branch'], safe='')
    head = quote(f"{repository.split('/')[0]}:{metadata['default_branch']}", safe='')
    comparison = fetch(f'repos/{upstream}/compare/{base}...{head}')
    try:
        release = fetch(f'repos/{upstream}/releases/latest')
    except subprocess.CalledProcessError as error:
        # A missing release is normal. Authentication, rate and server errors are not.
        if '(HTTP 404)' not in (error.stderr or ''):
            raise
        release = None
    release_comparison = None
    if release:
        tag = quote(release['tag_name'], safe='')
        release_comparison = fetch(f'repos/{upstream}/compare/{tag}...{head}')
    return dict(repository=repository, upstream=upstream,
                branch=metadata['default_branch'], status=comparison['status'],
                ahead=comparison['ahead_by'], behind=comparison['behind_by'],
                latest_release=release['tag_name'] if release else None,
                release_behind=release_comparison['behind_by'] if release_comparison else None)


def main():
    """Print a fork health report for each requested repository and exit non-zero on failure."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('repositories', nargs='+', help='Explicit owner/repository allowlist')
    args = parser.parse_args()
    failed = False
    for repository in args.repositories:
        try:
            print(json.dumps(report(repository), sort_keys=True))
        except (ValueError, KeyError, subprocess.SubprocessError) as error:
            # Do not print subprocess stderr, which may contain sensitive diagnostics.
            print(json.dumps(dict(repository=repository, error=type(error).__name__)))
            failed = True
    raise SystemExit(1 if failed else 0)


if __name__ == '__main__':
    main()
