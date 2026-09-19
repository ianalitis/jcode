#!/usr/bin/env bash
set -euo pipefail

repo_root=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
scratch_root="${JCODE_SCRATCH_DIR:-${TMPDIR:-/tmp}}"
mkdir -p "$scratch_root"
tmp=$(mktemp -d "$scratch_root/jcode-dev-cargo-cwd.XXXXXX")
trap 'rm -rf "$tmp"' EXIT

fixture="$tmp/repo with spaces"
sibling="$tmp/sibling worktree"
unrelated="$tmp/unrelated project"
mkdir -p "$fixture/scripts" "$fixture/crates/demo" "$tmp/bin" "$tmp/home" "$tmp/work" "$unrelated"
cp "$repo_root/scripts/dev_cargo.sh" "$fixture/scripts/dev_cargo.sh"
cp "$repo_root/scripts/remote_config.sh" "$fixture/scripts/remote_config.sh"
chmod +x "$fixture/scripts/dev_cargo.sh"

cat > "$fixture/Cargo.toml" <<'EOF'
[workspace]
members = ["crates/demo"]
EOF
cat > "$fixture/crates/demo/Cargo.toml" <<'EOF'
[package]
name = "demo"
version = "0.0.0"
EOF
cat > "$unrelated/Cargo.toml" <<'EOF'
[package]
name = "unrelated"
version = "0.0.0"
EOF

git -C "$fixture" init -q
git -C "$fixture" config user.email test@example.com
git -C "$fixture" config user.name 'Test User'
git -C "$fixture" add .
git -C "$fixture" commit -qm fixture
git -C "$fixture" worktree add -q --detach "$sibling" HEAD
git -C "$unrelated" init -q
ln -s "$fixture" "$tmp/repo link"

cat > "$tmp/bin/cargo" <<'EOF'
#!/usr/bin/env bash
{
  printf 'call\n'
  printf 'cwd=%s\n' "$(pwd -P)"
  printf 'argc=%s\n' "$#"
  for arg in "$@"; do
    printf 'arg=%s\n' "$arg"
  done
} >> "$FAKE_CARGO_LOG"
exit "${FAKE_CARGO_EXIT:-0}"
EOF
chmod +x "$tmp/bin/cargo"

export PATH="$tmp/bin:$PATH"
export HOME="$tmp/home"
export TMPDIR="$tmp/work"
export JCODE_DEV_CARGO_SCRIPT="$fixture/scripts/dev_cargo.sh"
export JCODE_RUST_ACTION_LOG=0
export JCODE_REMOTE_CARGO=0
export JCODE_CARGO_GATE=off
export JCODE_PARALLEL_FRONTEND=0
export JCODE_BUILD_GIT_HASH=test
export SCCACHE_DISABLE=1

cargo() {
  if [[ "${JCODE_IN_DEV_CARGO:-0}" == "1" ]]; then
    command cargo "$@"
  else
    JCODE_IN_DEV_CARGO=1 "$JCODE_DEV_CARGO_SCRIPT" "$@"
  fi
}
export -f cargo

run_case() {
  local name="$1" caller="$2" expected_cwd="$3" expected_status="$4"
  shift 4
  local log="$tmp/$name.log" expected="$tmp/$name.expected" status=0
  : > "$log"
  if (cd "$caller" && FAKE_CARGO_LOG="$log" FAKE_CARGO_EXIT="$expected_status" cargo "$@"); then
    status=0
  else
    status=$?
  fi

  if [[ "$status" -ne "$expected_status" ]]; then
    printf '%s: expected exit %s, got %s\n' "$name" "$expected_status" "$status" >&2
    exit 1
  fi
  {
    printf 'call\n'
    printf 'cwd=%s\n' "$(cd "$expected_cwd" && pwd -P)"
    printf 'argc=%s\n' "$#"
    for arg in "$@"; do
      printf 'arg=%s\n' "$arg"
    done
  } > "$expected"
  if ! cmp -s "$expected" "$log"; then
    printf '%s: fake Cargo cwd/arguments/call count differed\n' "$name" >&2
    diff -u "$expected" "$log" >&2 || true
    exit 1
  fi
}

# Invocations inside the wrapper's repository remain owned and run from its root.
run_case root "$fixture" "$fixture" 0 metadata --format-version 1
run_case nested "$tmp/repo link/crates/demo" "$fixture" 0 check --package 'demo crate'

# The inherited shell function must hand other checkouts to real Cargo in place.
run_case sibling "$sibling" "$sibling" 23 test --manifest-path 'crate path/Cargo.toml'
run_case unrelated "$unrelated" "$unrelated" 19 check --all-targets

echo 'dev_cargo cwd routing tests passed'
