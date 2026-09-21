# Repository Guidelines

## Repository Scope

- Jcode Desktop is in a separate repository.

## Development Workflow

- **Use the user's Git identity** - Create commits with the configured
  `user.name` and `user.email`. Do not override them with `Jcode`, `Jcode agent`,
  or a fabricated agent email. Preserve existing contributor attribution when
  integrating work. If no identity is configured, ask rather than inventing one.
- **Welcome pull requests from everyone** - Review contributions on their merits,
  regardless of whether the author is a maintainer, an existing contributor, a
  first-time contributor, or an agent. Good PRs can be merged directly after review
  and validation. Do not require a maintainer-authored rewrite merely because of
  who submitted the change. See `CONTRIBUTING.md` for the contribution policy.
- **Keep work scoped** - Work on your own branch and preserve unrelated work. When
  the user asks you to review or integrate a PR or branch, you may inspect, test,
  and integrate that contribution regardless of author status. Do not pull in
  unrelated branches or merge a PR without user authorization.

## Install Notes
- `~/.local/bin/jcode` is the launcher symlink used from `PATH`.
- `~/.jcode/builds/current/jcode` is the active local/source-build channel; self-dev builds and `scripts/install_release.sh` point the launcher here.
- `~/.jcode/builds/stable/jcode` is the stable release channel; `scripts/install.sh` installs this and points the launcher here.
- `~/.jcode/builds/versions/<version>/jcode` stores immutable binaries.
- `~/.jcode/builds/canary/jcode` still exists for canary/testing flows, but it is not the primary self-dev install path.
- On Windows, the equivalents are `%LOCALAPPDATA%\\jcode\\bin\\jcode.exe` for the launcher, `%LOCALAPPDATA%\\jcode\\builds\\stable\\jcode.exe` for stable, and `%LOCALAPPDATA%\\jcode\\builds\\versions\\<version>\\jcode.exe` for immutable installs; `scripts/install.ps1` currently installs the stable channel.
- Ensure `~/.local/bin` is **before** `~/.cargo/bin` in `PATH`.

## Integrating branches, stashes and worktrees

Do not rebuild a branch ledger by hand. `scripts/branch_ledger.sh` classifies
every local branch, stash and worktree against HEAD in about a minute and
prints a disposition table (`integrated`, `relanded`, `merge-ready`,
`conflicts`, `cherry`, `stale`); `docs/BRANCH_LEDGER_2026-09-21.md` is the
last run with the dispositions that were applied. Rules that follow from it:

- `git cherry` and subject matching decide whether work is present, not
  ancestry. Most "unmerged" branches here were already relanded by a rebase or
  by upstream; verify by a passing test name, then treat as integrated.
- `merge-ready` rows may be merged after their crate tests pass. `cherry` rows
  (thousands of commits behind) are never merged: cherry-pick the unique
  commits, or port by hand onto the split test layout when the pick conflicts
  only with our own reorganisation.
- Deleting a branch, worktree or stash always needs operator approval, even
  for `integrated` rows.
- One writer per repository: check `scripts/repo-lease.sh status` (in
  `~/dotfiles/scripts`) before mutating, and commit with
  `git commit --only -- <paths>` so a concurrent session's staged files never
  land in your commit.

## Bounding commands

There is no GNU `timeout` on macOS, and unbounded `git merge-tree` or `cargo
test` calls have hung sessions for twenty minutes or more. Wrap anything that
can wedge in `scripts/bounded.sh <secs> <cmd>`; it exits 124 on expiry like GNU
timeout. After `git apply`, `git merge` or `git stash pop`, `touch` the touched
sources before trusting `cargo test` or `cargo check --all-targets`: cargo has
run stale test binaries and reported phantom errors for symbols that exist.

## Verifying a change at runtime

`cargo build` alone proves nothing about behavior. `jcode run` and interactive
sessions are served by the long-lived daemon at
`~/.jcode/builds/shared-server/jcode`, which is a symlink into
`~/.jcode/builds/versions/<version>/`. Until that symlink is repointed and the
daemon restarted (`jcode self-dev --build`), a freshly built binary is inert and
every runtime check silently measures the old code.

To test a change without disturbing the shared daemon or the caller's session,
run your build against its own socket:

```bash
cargo build --profile selfdev
./target/selfdev/jcode run --no-update --socket /run/user/1000/jcode-mytest.sock '<prompt>'
```

Two things that waste time otherwise:

- `crate::logging::info` writes to a log file, not stderr, so instrumenting a
  code path with it produces no visible output under `--trace`. Use `eprintln!`
  for throwaway diagnostics and delete it before committing.
- Confirm which binary you are actually inspecting. `strings` on
  `builds/shared-server/jcode` reads a 70-byte symlink, not a program; resolve it
  with `readlink -f` first.
