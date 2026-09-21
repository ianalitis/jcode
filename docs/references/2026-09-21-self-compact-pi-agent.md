# Reference: self-compacting Pi agent (IndyDevDan, 2026-09)

Sources: video `https://www.youtube.com/watch?v=3b0U4_02bAE` and repository
`https://github.com/disler/self-compact-pi-agent` (MIT). Raw transcript and a
shallow clone are in `$JCODE_SCRATCH_DIR/selfcompact/` (scratch, not committed).
This file is a paraphrased digest for our own planning; it is not a copy of
either source.

## The problem as stated

- Every turn re-sends the whole history. Competence decays as the window fills
  ("context rot") and each call costs more than the last.
- Built-in auto-compaction in every harness is reactive: it fires near the wall
  with a generic summary prompt, gives the agent no warning, and does not carry
  the agent's own "what I was about to do" across the boundary. A bystander's
  summary is how finished work gets redone.
- For out-loop swarms (10 to 100s of agents for hours) this is the dominant
  cost and failure mode. In the video, a GLM 5.2 run died at 98% context on a
  task that a self-compacting agent would have finished.

## The mechanism (one Pi extension, ~1.6k lines TS)

| Phase | Trigger | Agent sees | Tools |
|---|---|---|---|
| notice | usage > soft line (default 10%) | one transient hint with live numbers | all |
| warning | usage > warning line (20%) | "compact soon, hard cutoff at N", re-injected every call | all |
| forced | usage > warning + buffer (30%, capped 90%) | every tool call rejected with a reason naming `self_compact` | only `self_compact` |
| handoff | agent calls `self_compact({note_to_self})` | note saved, run ends, compaction once idle, note returned verbatim as the next message | restored |

Key design points worth copying:

1. **The note is the contract.** Byte-for-byte round trip. Summary and note are
   separate artifacts; the summary never replaces the note.
2. **Threshold guidance never persists.** Injected into the next model call and
   dropped, so it does not itself consume context.
3. **`view_context()` tool** returns used tokens, percent, thresholds and
   remaining as JSON so the agent never guesses where it stands.
4. **Custom compaction prompt** (structured: Goal / Constraints / Done /
   In progress / Blocked / Decisions / Next steps / Critical context /
   read-files / modified-files) with the rule "never invent completed work;
   mark verified vs unverified".
5. **Failure keeps the lock.** Failed or cancelled compaction preserves the note
   and keeps other tools blocked until success; reload/resume/`/tree` rebuild
   the handoff from the session; an answered handoff never restarts.
6. **HIL commands** `/self-compact-info` (no model turn) and `/self-compact-now`
   (reuses a saved note on retry). Built-in `/compact` untouched.
7. **Deterministic e2e**: real `pi --mode rpc` with a scripted provider whose
   reported usage grows every turn, so every crossing, lock, and note
   round-trip is testable for free.
8. **Tuning guidance**: wide gap between notice and warning, short gap between
   warning and forced. Keep every threshold above Pi's `keepRecentTokens`
   (20k) or there is nothing to cut.

Known limits the author lists: usage is an estimate and a single huge tool
result can overshoot; running tools finish after the lock; a model can ignore
the warning (hence forced); compaction quality is model quality.

## Process observations from the video

- Author writes the requirements by hand first (`prompts/end_draft_plan.md`),
  then hands the same packet to three harness/model pairs (Claude Fable 5.1,
  GPT-6 Astra via Codex, GLM 5.2 via Pi) and compares. Fable wrote the best
  prompts; Astra was most token-efficient (136k vs 500k); GLM ran out of context.
- Definition of Done is the grading rubric, including instant-fail rules for
  reading other agents' work dirs. This is the same "packet with acceptance
  spec" shape our `policy/goal-packet.md` uses.

## What jcode already has (so we do not rebuild)

- `crates/jcode-base/src/compaction.rs` + `jcode-compaction-core`: reactive
  (80%), proactive (EWMA lookahead), semantic (embedding topic shift) modes,
  hard-threshold synchronous compact, emergency payload truncation, transfer
  compaction state for session handoff, OpenAI native compaction.
- `Agent::request_manual_compaction` and `/compact`.
- Missing relative to the reference: an agent-callable `self_compact(note)`
  tool, a `view_context` tool, threshold-driven transient guidance, a forced
  phase that blocks other tools, a verbatim note round trip, and the
  structured summary prompt as an operator-editable file.
- Pi side: `crates/jcode-executor-pi` drives `pi --mode rpc` (no tools). The
  reference extension is loadable per launch with `-e`, so the inner-loop Pi
  worker can adopt it as-is without a jcode code change.
