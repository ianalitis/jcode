# PR body draft: restore the Format gate on master (2026-10-09)

Status: **not posted**. Opening the PR is an operator decision; this is the text
to paste, with `patches-master-format-drift-2026-10-09.patch` applied as the
commit.

---

## Restore the `Format` job on master (three rustfmt hunks, two files)

`Format` fails on master at `a6ba7844f`:

```
python3 scripts/check_module_files.py   # OK
cargo fmt --all -- --check              # exit 1
  Diff in crates/jcode-provider-openai-runtime/src/lib.rs:966
  Diff in crates/jcode-tui/src/tui/app/onboarding_flow_control.rs:633
  Diff in crates/jcode-tui/src/tui/app/onboarding_flow_control.rs:874
```

Exactly three hunks in two files, nothing else in the tree (1360 `.rs` files).
The patch is rustfmt's own output, not hand-written formatting judgement: after
applying it, `cargo fmt --all -- --check` exits 0 and the module check still
passes.

Which commit introduced each, by bisecting with `rustfmt --check` over the
candidate trees:

- `lib.rs:966` is clean at `f5d963a4e` and dirty at `bda5f3d9c` (2026-10-06),
  the hunk reported in #1740.
- Both `onboarding_flow_control.rs` hunks are clean at `7df55c30f` and dirty at
  `72faaf145` (2026-10-08T23:08:28-07:00).

That second date is after PR #1764's head (`13375d186`, 21:28:14-07:00), and
`git diff 21eb960a2 13375d186 -- crates/jcode-tui/src/tui/app/onboarding_flow_control.rs`
is empty. #1764 therefore no longer restores the gate on its own: its own run
already fails `Quality Guardrails` at the oversized-file ratchet step while its
`Check formatting` step passes. This change is independent of #1764 and can land
before or after it.

Reproduce:

```sh
git archive origin/master | tar -x -C /tmp/master-tree && cd /tmp/master-tree
command cargo fmt --all -- --check            # exit 1, three hunks
patch -p1 < patches-master-format-drift-2026-10-09.patch
command cargo fmt --all -- --check            # exit 0
```

(The `command` prefix matters only if you have the repo's `dev_cargo` shell
shim installed; it `cd`s to the checkout when the caller is inside it.)

Fixes the formatting half of #1740. Happy for this to be squashed into whatever
PR lands first; it is a pure `cargo fmt --all` application against `a6ba7844f`.
