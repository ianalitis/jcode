# Configuring the System Prompt

jcode builds its system prompt from several layers. Two of them are user-editable
files, so you can tune agent behavior without rebuilding.

## Layers (in order)

1. **Base system prompt** — built-in `crates/jcode-base/src/prompt/system_prompt.md`,
   overridable by file (see below).
2. Capability modules (e.g. Mermaid guidance).
3. Product-specific self-dev guidance. Sessions rooted in a Jcode Desktop
   checkout automatically receive the Desktop prompt and `desktop_selfdev` tool,
   separate from CLI/TUI self-dev flags, `selfdev`, and `debug_socket`.
4. `AGENTS.md` — global instructions first, then repository instructions from the
   repository root through the working directory.
5. Prompt overlay — `./.jcode/prompt-overlay.md` and `~/.jcode/prompt-overlay.md`.
6. Preferred tools — `./.jcode/preferred-tools.md` and `~/.jcode/preferred-tools.md`.
7. Memory and the active skill prompt (dynamic, not cached).

## Adding guidance (most common)

Append instructions without touching the default prompt:

- `~/.jcode/prompt-overlay.md` — applies everywhere.
- `./.jcode/prompt-overlay.md` — applies to one project.

Both are included when present. For layers 4–6, if the project and global paths
resolve to the same canonical path (for example, when working in `$HOME` or using
symlink aliases), the file is included once under its project heading. Distinct
files are still both included, even when their contents match. The global
`.jcode` directory respects `JCODE_HOME` when set.

## Repository instructions (`AGENTS.md`)

`AGENTS.md` discovery is deliberately bounded:

1. Load the global `~/AGENTS.md` selected by jcode.
2. In a Git repository, resolve Git's canonical worktree root and load each
   `AGENTS.md` on the direct root-to-working-directory ancestor chain, broadest
   to most specific. Repository discovery ignores inherited `GIT_DIR` and
   `GIT_WORK_TREE` overrides so a parent process cannot redirect that root.
3. Outside a Git repository, load only the working directory's `AGENTS.md`
   after the global file.

Files above the Git worktree root, in sibling directories, or under an inferred
outer workspace are not loaded. An explicit outer instruction-root feature is
not currently configured or inferred.

Each accepted section identifies its source path, layer, and SHA-256 content
hash. Canonical file identity removes duplicates. Project instruction symlinks
must resolve inside the canonical repository root (or the canonical working
directory outside a repository). The explicitly selected global file preserves
the existing behavior of allowing a user-selected symlink target outside the
project boundary.

Only regular UTF-8 files are accepted. Discovery accepts at most 32 unique files,
64 KiB per file, and 256 KiB total. A file that would exceed a limit is rejected
whole rather than truncated. Reads use the opened file's metadata and consume at
most 64 KiB plus one detection byte. Invalid UTF-8, unreadable files, non-regular
files, escaping project symlinks, and limit violations produce bounded diagnostics
with a path and reason. Diagnostics do not include rejected file contents or raw
I/O errors.

Containment is a snapshot-time, best-effort filesystem check. jcode canonicalizes
the candidate and verifies its boundary before opening it, then verifies that the
opened handle is a regular file. It does not claim protection against a hostile
process concurrently replacing path components between those operations; use a
workspace whose instruction paths are not being adversarially mutated during
session capture.

## Replacing the base prompt

To fully replace layer 1, create either file:

- `./.jcode/system-prompt.md` (project, highest precedence)
- `~/.jcode/system-prompt.md` (global)

The first non-empty file wins; otherwise the built-in default is used. An empty or
whitespace-only file falls back to the default, so you cannot accidentally ship an
empty prompt.

This replaces only the base prompt. AGENTS.md, overlays, skills, and memory still apply.

## Notes

- AGENTS.md uses the prompt's existing captured `(content, context-info)` snapshot.
  A stable workspace snapshot does not watch files for edits. New sessions,
  working-directory changes, clears, and restored/resumed session setup recapture
  the applicable files. Other prompt inputs have their own lifecycles, so this is
  not a claim that every session input is immutable across those transitions.
- Editing the built-in `system_prompt.md` requires a rebuild (`selfdev build-reload`),
  since it is embedded with `include_str!`.
- Swarm model-routing guidance has its own analogous file: `.jcode/swarm-prompt.md`.
  Use `/swarm-prompt` to edit the active project or global file. New agents load
  the latest contents immediately; already-running agents keep the prompt they
  captured at session creation so their tool definition and context cache stay stable.
