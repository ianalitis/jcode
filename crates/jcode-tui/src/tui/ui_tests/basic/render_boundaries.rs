// Prepared-body boundary invariants.
//
// `PreparedMessages::message_boundaries` is the index that makes incremental
// rendering fast: `matching_prefix_len` compares
// `message_boundaries[k].msg_hash` against `display_messages[k].stable_cache_hash()`
// positionally, and `truncate_prepared_to_boundary` cuts every parallel array
// at `message_boundaries[k - 1]`. The whole reuse path therefore rests on one
// invariant:
//
//   message_boundaries.len() == display_messages.len(), in transcript order.
//
// A message that renders no lines (a pinned todo card, an unknown role) must
// still record a zero-length segment. A missing segment shifts every later
// boundary by one, so the next append can only reuse the prefix up to that
// message, the incremental base's boundary count no longer matches the cached
// message count, and reuse stitches a misaligned table.
//
// These tests assert the invariant directly, sweep every cut point of the
// reuse flow, and repeat both over randomized multi-kind transcripts so a new
// render path cannot reintroduce the desync.

/// RAII guard for the `JCODE_PIN_TODOS` env override. Mirrors the guard in
/// `tui::app::tests::todo_card` (which owns the pinned-band tests): jcode-base
/// throttles config env re-checks unless jcode-base itself is the crate under
/// test, so force a reload on set and on drop or a sibling test's pin_todos
/// state leaks into this one for up to 500ms.
struct PinTodosGuard;

impl PinTodosGuard {
    fn enable() -> Self {
        crate::env::set_var("JCODE_PIN_TODOS", "1");
        crate::config::invalidate_config_cache();
        Self
    }

    /// `display.pin_todos` defaults to true, so the inline-card path needs an
    /// explicit override rather than the absence of one.
    fn disable() -> Self {
        crate::env::set_var("JCODE_PIN_TODOS", "0");
        crate::config::invalidate_config_cache();
        Self
    }
}

impl Drop for PinTodosGuard {
    fn drop(&mut self) {
        crate::env::remove_var("JCODE_PIN_TODOS");
        crate::config::invalidate_config_cache();
    }
}

fn line_to_plain(line: &Line) -> String {
    line.spans.iter().map(|s| s.content.as_ref()).collect()
}

/// Assert `prepared` records exactly one boundary per display message, in
/// transcript order, with per-message line counts that tile the parallel
/// arrays. This is the invariant the prefix/suffix reuse selectors depend on;
/// assert it at every intermediate step of a reuse flow, not just at the end.
fn assert_boundaries_align(prepared: &PreparedMessages, messages: &[DisplayMessage], ctx: &str) {
    assert_eq!(
        prepared.message_boundaries.len(),
        messages.len(),
        "{ctx}: one boundary per display message (a message that renders \
         nothing must still record a zero-length segment)"
    );
    for (idx, (boundary, msg)) in prepared.message_boundaries.iter().zip(messages).enumerate() {
        assert_eq!(
            boundary.msg_hash,
            msg.stable_cache_hash(),
            "{ctx}: boundary {idx} must hash the message at display_messages[{idx}]"
        );
    }

    let mut previous: Option<MessageBoundary> = None;
    for (idx, boundary) in prepared.message_boundaries.iter().enumerate() {
        assert!(
            boundary.wrapped_len <= prepared.wrapped_lines.len(),
            "{ctx}: boundary {idx} wrapped_len {} exceeds {} wrapped lines",
            boundary.wrapped_len,
            prepared.wrapped_lines.len()
        );
        assert!(
            boundary.raw_len <= prepared.raw_plain_lines.len(),
            "{ctx}: boundary {idx} raw_len {} exceeds {} raw lines",
            boundary.raw_len,
            prepared.raw_plain_lines.len()
        );
        assert!(
            boundary.user_prompt_len <= prepared.user_prompt_texts.len(),
            "{ctx}: boundary {idx} user_prompt_len {} exceeds {} prompts",
            boundary.user_prompt_len,
            prepared.user_prompt_texts.len()
        );
        if let Some(previous) = previous {
            assert!(
                boundary.wrapped_len >= previous.wrapped_len
                    && boundary.raw_len >= previous.raw_len
                    && boundary.user_prompt_len >= previous.user_prompt_len,
                "{ctx}: boundary {idx} must not move backwards"
            );
        }
        previous = Some(*boundary);
    }

    // The final boundary owns the whole body: nothing may be rendered outside
    // a message (streaming text is appended after this point and is never part
    // of a reusable segment, so it is not observable on this path).
    match prepared.message_boundaries.last() {
        Some(last) => {
            assert_eq!(
                last.wrapped_len,
                prepared.wrapped_lines.len(),
                "{ctx}: last boundary must cover every wrapped line"
            );
            assert_eq!(
                last.raw_len,
                prepared.raw_plain_lines.len(),
                "{ctx}: last boundary must cover every raw line"
            );
            assert_eq!(
                last.user_prompt_len,
                prepared.user_prompt_texts.len(),
                "{ctx}: last boundary must cover every user prompt"
            );
        }
        None => {
            assert!(
                prepared.wrapped_lines.is_empty()
                    && prepared.raw_plain_lines.is_empty()
                    && prepared.user_prompt_texts.is_empty(),
                "{ctx}: an empty transcript must prepare an empty body"
            );
        }
    }
}

/// Assert two prepared bodies are byte-for-byte equivalent across every
/// observable array. Used to prove prefix-reuse output matches a fresh full
/// build. `messages` is the transcript both bodies were built for: reuse may
/// only be accepted when the boundary table also lands on that transcript, so
/// alignment is asserted for both sides here rather than in each caller.
fn assert_prepared_equivalent(
    a: &PreparedMessages,
    b: &PreparedMessages,
    messages: &[DisplayMessage],
    ctx: &str,
) {
    assert_boundaries_align(a, messages, &format!("{ctx}/reused"));
    assert_boundaries_align(b, messages, &format!("{ctx}/full"));

    let a_lines: Vec<String> = a.wrapped_lines.iter().map(line_to_plain).collect();
    let b_lines: Vec<String> = b.wrapped_lines.iter().map(line_to_plain).collect();
    assert_eq!(a_lines, b_lines, "{ctx}: wrapped_lines text differ");
    assert_eq!(
        a.wrapped_plain_lines, b.wrapped_plain_lines,
        "{ctx}: wrapped_plain_lines differ"
    );
    assert_eq!(
        a.wrapped_copy_offsets, b.wrapped_copy_offsets,
        "{ctx}: wrapped_copy_offsets differ"
    );
    assert_eq!(
        a.raw_plain_lines, b.raw_plain_lines,
        "{ctx}: raw_plain_lines differ"
    );
    assert_eq!(
        a.user_prompt_texts, b.user_prompt_texts,
        "{ctx}: user_prompt_texts differ"
    );
    assert_eq!(
        a.wrapped_user_indices, b.wrapped_user_indices,
        "{ctx}: wrapped_user_indices differ"
    );
    assert_eq!(
        a.wrapped_user_prompt_starts, b.wrapped_user_prompt_starts,
        "{ctx}: wrapped_user_prompt_starts differ"
    );
    assert_eq!(
        a.wrapped_user_prompt_ends, b.wrapped_user_prompt_ends,
        "{ctx}: wrapped_user_prompt_ends differ"
    );
    let a_map: Vec<usize> = a.wrapped_line_map.iter().map(|m| m.raw_line).collect();
    let b_map: Vec<usize> = b.wrapped_line_map.iter().map(|m| m.raw_line).collect();
    assert_eq!(a_map, b_map, "{ctx}: wrapped_line_map raw_line differ");
    assert_eq!(
        a.image_regions.len(),
        b.image_regions.len(),
        "{ctx}: image_regions count differ"
    );
    for (x, y) in a.image_regions.iter().zip(b.image_regions.iter()) {
        assert_eq!(
            x.abs_line_idx, y.abs_line_idx,
            "{ctx}: image_region abs_line_idx differ"
        );
        assert_eq!(x.end_line, y.end_line, "{ctx}: image_region end_line differ");
    }
    assert_eq!(
        a.edit_tool_ranges.len(),
        b.edit_tool_ranges.len(),
        "{ctx}: edit_tool_ranges count differ"
    );
    for (x, y) in a.edit_tool_ranges.iter().zip(b.edit_tool_ranges.iter()) {
        assert_eq!(
            x.start_line, y.start_line,
            "{ctx}: edit_tool_range start_line differ"
        );
        assert_eq!(x.end_line, y.end_line, "{ctx}: edit_tool_range end_line differ");
    }
    assert_eq!(
        a.copy_targets.len(),
        b.copy_targets.len(),
        "{ctx}: copy_targets count differ"
    );
    for (x, y) in a.copy_targets.iter().zip(b.copy_targets.iter()) {
        assert_eq!(
            x.start_line, y.start_line,
            "{ctx}: copy_target start_line differ"
        );
        assert_eq!(x.end_line, y.end_line, "{ctx}: copy_target end_line differ");
    }
    let a_b: Vec<_> = a
        .message_boundaries
        .iter()
        .map(|b| (b.msg_hash, b.wrapped_len, b.raw_len, b.user_prompt_len))
        .collect();
    let b_b: Vec<_> = b
        .message_boundaries
        .iter()
        .map(|b| (b.msg_hash, b.wrapped_len, b.raw_len, b.user_prompt_len))
        .collect();
    assert_eq!(a_b, b_b, "{ctx}: message_boundaries differ");
}

/// Reuse must resume from *every* cut point and land on the full build.
///
/// For each `k`, the body for the whole transcript is truncated to its first
/// `k` boundaries (exactly what `build_body_from_base` does when the matching
/// prefix is `k`) and the tail is re-rendered incrementally. Two properties are
/// asserted: the result equals a fresh full build, and the reuse path never
/// degrades (every `k < len` must take the `incremental` path, `k == len` the
/// `prefix_exact` path). A boundary table that drifts by one fails both: the
/// output differs and the cut lands inside another message.
fn assert_every_cut_point_resumes(label: &str, state: &TestState, width: u16) {
    let messages = state.display_messages.clone();
    let full = super::prepare::prepare_body(state, width, false);
    assert_boundaries_align(&full, &messages, &format!("{label}/full"));

    for k in 0..=messages.len() {
        let mut base = Arc::new(full.clone());
        super::prepare::truncate_prepared_to_boundary(Arc::make_mut(&mut base), k);
        assert_boundaries_align(
            base.as_ref(),
            &messages[..k],
            &format!("{label}/truncate_{k}"),
        );

        let (reused, path) = super::prepare::build_body_from_base(state, width, base, k, 0, messages.len());
        let expected = if k == messages.len() {
            "prefix_exact"
        } else {
            "incremental"
        };
        assert_eq!(
            path, expected,
            "{label}: cut at {k}/{} must resume through {expected}, not {path}",
            messages.len()
        );
        assert_prepared_equivalent(&reused, &full, &messages, &format!("{label}/cut_{k}"));
    }
}

fn state_with_messages(messages: Vec<DisplayMessage>) -> TestState {
    TestState {
        display_messages: messages,
        messages_version: 1,
        ..Default::default()
    }
}

/// Prepend sweep: the front-truncation mirror of `assert_every_cut_point_resumes`,
/// and the only sweep that reaches `prepare_body_prepended`.
///
/// When older compacted history is revealed above an unchanged tail the new
/// transcript is `revealed head + fresh compaction marker + unchanged tail`
/// while the cached base is `old marker + unchanged tail`, so the prefix match
/// collapses at index 0 and the suffix match covers the whole tail: the
/// selector takes the `suffix_reuse` branch and stitches
/// `head boundaries + base.message_boundaries[drop_msgs..]` with shifted line
/// counts. That reassembly is the *second* producer of `message_boundaries`
/// (the first being `render_message_into`'s segment table), and it can only stay
/// aligned if the base recorded exactly one boundary per message. A base that
/// skipped a message shifts every stitched boundary, so the reused body stops
/// matching a full build and later reuse cuts inside another message.
fn assert_every_front_cut_point_resumes(label: &str, tail: &[DisplayMessage], width: u16) {
    assert!(tail.len() >= 2, "{label}: need a multi-message tail to reuse");
    for drop in 1..tail.len() {
        // Non-user head messages keep prompt-number arithmetic trivially
        // invariant (no prompts revealed, offsets unchanged), so the only thing
        // under test here is the boundary reassembly.
        let revealed = [
            DisplayMessage::meta(format!("revealed {drop}")),
            DisplayMessage::assistant(format!("older assistant turn {drop}")),
        ];
        let base_state = state_with_messages(
            std::iter::once(DisplayMessage::meta("compacted history"))
                .chain(tail[drop..].iter().cloned())
                .collect(),
        );
        let new_state = state_with_messages(
            revealed
                .iter()
                .cloned()
                .chain(std::iter::once(DisplayMessage::meta(
                    "compacted history (revealed)",
                )))
                .chain(tail[drop..].iter().cloned())
                .collect(),
        );
        let ctx = format!("{label}/front_drop_{drop}");

        let base = Arc::new(super::prepare::prepare_body(&base_state, width, false));
        assert_boundaries_align(
            &base,
            &base_state.display_messages,
            &format!("{ctx}/base"),
        );
        let full = super::prepare::prepare_body(&new_state, width, false);
        let prev_count = base.message_boundaries.len();
        let (reused, path) = super::prepare::build_body_from_base(
            &new_state,
            width,
            base,
            prev_count,
            0,
            new_state.display_messages.len(),
        );
        assert_eq!(
            path, "suffix_reuse",
            "{ctx}: revealing history above an unchanged tail must stitch the \
             cached suffix, not rebuild ({path})"
        );
        assert_prepared_equivalent(&reused, &full, &new_state.display_messages, &ctx);
    }
}

/// Deterministic PRNG: a failing seed is reproducible from the panic message
/// and no wall-clock entropy leaks into the test.
struct Lcg(u64);

impl Lcg {
    fn new(seed: u64) -> Self {
        Self(seed.wrapping_mul(6364136223846793005).wrapping_add(1))
    }

    fn next(&mut self) -> u64 {
        self.0 = self
            .0
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        self.0 >> 33
    }

    fn pick(&mut self, n: usize) -> usize {
        (self.next() % n as u64) as usize
    }
}

const RNG_WORDS: [&str; 8] = [
    "alpha", "boundary", "cache", "delta", "epoch", "fence", "graph", "hash",
];

fn random_text(rng: &mut Lcg, min_words: usize, max_words: usize) -> String {
    let count = min_words + rng.pick(max_words - min_words + 1);
    let mut text = String::new();
    for _ in 0..count {
        if !text.is_empty() {
            text.push(' ');
        }
        text.push_str(RNG_WORDS[rng.pick(RNG_WORDS.len())]);
    }
    text
}

fn random_tool_call(idx: usize, name: &str) -> ToolCall {
    let input = match name {
        "edit" => serde_json::json!({
            "file_path": format!("src/file_{idx}.rs"),
            "old_string": "let old = 1;",
            "new_string": "let new = 2;",
        }),
        "todo" => serde_json::json!({"action": "update", "id": format!("t{idx}")}),
        _ => serde_json::json!({"file_path": format!("src/file_{idx}.rs")}),
    };
    ToolCall {
        id: format!("call-{name}-{idx}"),
        name: name.to_string(),
        input,
        intent: None,
        thought_signature: None,
    }
}

/// One message of a randomly chosen kind, covering every render arm in
/// `render_message_into` (including the arms that render nothing).
fn random_message(rng: &mut Lcg, idx: usize) -> DisplayMessage {
    match rng.pick(16) {
        0 => DisplayMessage::user(random_text(rng, 3, 12)),
        1 => DisplayMessage::assistant(random_text(rng, 2, 20)),
        2 => DisplayMessage::assistant_with_duration(random_text(rng, 2, 10), 1.5),
        3 => DisplayMessage::assistant(format!(
            "```rust\nfn f{idx}() {{}}\n```\n\n{}",
            random_text(rng, 2, 8)
        )),
        4 => DisplayMessage::tool("read out.txt", random_tool_call(idx, "read")),
        5 => DisplayMessage::tool("edited file", random_tool_call(idx, "edit")),
        6 => DisplayMessage::tool("Todos updated", random_tool_call(idx, "todo")),
        7 => DisplayMessage::tool_text(random_text(rng, 2, 8)),
        8 => DisplayMessage::meta(random_text(rng, 1, 6)),
        9 => DisplayMessage::system(random_text(rng, 1, 6)),
        10 => DisplayMessage::swarm(
            "Plan graph · rng",
            "swarm update\n\n- step one\n- step two",
        ),
        11 => DisplayMessage::memory("memory", random_text(rng, 2, 6)),
        12 => DisplayMessage::error(random_text(rng, 2, 6)),
        13 => DisplayMessage::spacer(1 + rng.pick(2)),
        14 => DisplayMessage {
            role: "reasoning".to_string(),
            content: random_text(rng, 2, 8),
            tool_calls: Vec::new(),
            duration_secs: None,
            title: None,
            tool_data: None,
        },
        _ => DisplayMessage {
            role: "unknown-role".to_string(),
            content: random_text(rng, 2, 8),
            tool_calls: Vec::new(),
            duration_secs: None,
            title: None,
            tool_data: None,
        },
    }
}

fn random_transcript(rng: &mut Lcg, len: usize) -> TestState {
    let messages = (0..len).map(|idx| random_message(rng, idx)).collect();
    TestState {
        display_messages: messages,
        messages_version: 1,
        ..Default::default()
    }
}

/// One transcript containing every render arm exactly once, including the two
/// that produce no lines of their own: the pinned todo card (skipped by config)
/// and an unknown role (no match arm). Both must still record a boundary.
fn all_kinds_transcript() -> TestState {
    let mut messages = vec![
        DisplayMessage::user("run the whole suite"),
        DisplayMessage::assistant("starting with the boundary invariants"),
        DisplayMessage::assistant_with_duration("cached answer", 0.4),
        DisplayMessage::assistant("```rust\nlet x = 1;\n```\n\nwith a code block"),
        DisplayMessage::tool("read a file", random_tool_call(0, "read")),
        DisplayMessage::tool("edited a file", random_tool_call(1, "edit")),
        DisplayMessage::tool("Todos updated", random_tool_call(2, "todo")),
        DisplayMessage::tool(
            "patched a file",
            ToolCall {
                id: "call-patch".to_string(),
                name: "apply_patch".to_string(),
                input: serde_json::json!({
                    "patch_text": "*** Begin Patch\n*** Update File: src/lib.rs\n@@\n-let a = 1;\n+let a = 2;\n*** End Patch\n"
                }),
                intent: Some("Apply the boundary fix".to_string()),
                thought_signature: None,
            },
        ),
        DisplayMessage::tool_text("tool output without tool data"),
        DisplayMessage::meta("meta note"),
        DisplayMessage::system("system note"),
        DisplayMessage::spacer(2),
        DisplayMessage::background_task("background task finished"),
        DisplayMessage::swarm("Plan graph · kinds", "plan\n\n- one\n- two"),
        DisplayMessage::memory("memory", "remember the boundary invariant"),
        DisplayMessage::usage("tokens: 12"),
        DisplayMessage::overnight("overnight progress"),
        DisplayMessage::todos(
            r#"[{"content":"keep one boundary per message","status":"in_progress","priority":"high","id":"t1"}]"#,
        ),
        DisplayMessage::error("something failed"),
        DisplayMessage {
            role: "reasoning".to_string(),
            content: "considering the cut points".to_string(),
            tool_calls: Vec::new(),
            duration_secs: None,
            title: None,
            tool_data: None,
        },
        DisplayMessage {
            role: "unknown-role".to_string(),
            content: "no match arm renders this".to_string(),
            tool_calls: Vec::new(),
            duration_secs: None,
            title: None,
            tool_data: None,
        },
        DisplayMessage::user("and the closing prompt"),
        DisplayMessage::assistant("done"),
    ];
    // Keep the todo tool message off the very first index so the cut sweep also
    // exercises a skip that is not the head message.
    messages.swap(2, 6);
    TestState {
        display_messages: messages,
        messages_version: 1,
        ..Default::default()
    }
}

/// Every message records exactly one boundary, including the ones that render
/// almost nothing: the pinned todo card (skipped by config, records a
/// zero-length segment) and an unknown role (no match arm, records only its
/// separator blank).
#[test]
fn test_render_boundaries_cover_every_message_kind() {
    let _env_lock = crate::storage::lock_test_env();
    let _pin = PinTodosGuard::enable();
    assert!(
        crate::config::config().display.pin_todos,
        "JCODE_PIN_TODOS must take effect for this test to exercise the skip path"
    );

    let state = all_kinds_transcript();
    let prepared = super::prepare::prepare_body(&state, 80, false);
    assert_boundaries_align(&prepared, &state.display_messages, "all_kinds");

    // The pinned todo message renders nothing at all, so its segment must be
    // zero-length rather than borrowing its neighbour's lines. It sits at
    // index 2 (`all_kinds_transcript` swaps it off the head so the cut sweep
    // also covers a skip that is not the first message).
    let skipped = 2usize;
    let boundary = prepared.message_boundaries[skipped];
    let previous = prepared.message_boundaries[skipped - 1];
    assert_eq!(
        (boundary.wrapped_len, boundary.raw_len, boundary.user_prompt_len),
        (
            previous.wrapped_len,
            previous.raw_len,
            previous.user_prompt_len
        ),
        "message {skipped} is skipped by pin_todos, so its boundary must be zero-length"
    );
}

/// Cut sweep over a transcript that mixes every message kind: each cut point
/// must resume incrementally onto the full build.
#[test]
fn test_every_cut_point_resumes_for_every_message_kind() {
    let _env_lock = crate::storage::lock_test_env();
    let _pin = PinTodosGuard::enable();

    let state = all_kinds_transcript();
    assert_every_cut_point_resumes("all_kinds", &state, 80);
    assert_every_cut_point_resumes("all_kinds_narrow", &state, 44);
}

/// The same cut sweep over randomized transcripts. Random kinds catch a
/// desync that only appears in a specific neighbour combination (e.g. a
/// zero-length segment followed by a wrapped card), which the fixed transcript
/// may not happen to build.
#[test]
fn test_every_cut_point_resumes_for_randomized_transcripts() {
    let _env_lock = crate::storage::lock_test_env();
    let _pin = PinTodosGuard::enable();

    for seed in [7u64, 11, 23, 42, 1337] {
        let mut rng = Lcg::new(seed);
        let len = 8 + rng.pick(7);
        let state = random_transcript(&mut rng, len);
        let width = 40 + rng.pick(60) as u16;
        assert_every_cut_point_resumes(&format!("rng_seed_{seed}"), &state, width);
    }
}

/// Front sweep over a transcript that mixes every message kind, with the pinned
/// todo card in play: revealing history above it must still stitch onto the full
/// build with one boundary per message.
#[test]
fn test_every_front_cut_point_resumes_for_every_message_kind() {
    let _env_lock = crate::storage::lock_test_env();
    let _pin = PinTodosGuard::enable();

    let tail = all_kinds_transcript().display_messages;
    assert_every_front_cut_point_resumes("all_kinds", &tail, 80);
    assert_every_front_cut_point_resumes("all_kinds_narrow", &tail, 44);
}

/// The same front sweep over randomized tails, so the reassembly is exercised
/// over neighbour combinations the fixed transcript does not build.
#[test]
fn test_every_front_cut_point_resumes_for_randomized_transcripts() {
    let _env_lock = crate::storage::lock_test_env();
    let _pin = PinTodosGuard::enable();

    for seed in [7u64, 11, 23, 42, 1337] {
        let mut rng = Lcg::new(seed);
        let len = 6 + rng.pick(6);
        let tail = (0..len).map(|idx| random_message(&mut rng, idx)).collect::<Vec<_>>();
        let width = 40 + rng.pick(60) as u16;
        assert_every_front_cut_point_resumes(&format!("rng_seed_{seed}"), &tail, width);
    }
}

/// Front sweep with the pinned band disabled. Inline todo cards are dropped
/// from the tail here: a rendered todo card embeds a "what changed" delta
/// computed from neighbouring messages, which `suffix_reuse_compatible`
/// documents as intentionally allowed to go stale, so it is not part of this
/// equivalence assertion. The zero-length-segment case is covered by the pinned
/// sweep above and by the tail cut sweeps.
#[test]
fn test_every_front_cut_point_resumes_without_pinned_todos() {
    let _env_lock = crate::storage::lock_test_env();
    let _pin = PinTodosGuard::disable();
    assert!(
        !crate::config::config().display.pin_todos,
        "this test must run with the todo card rendered inline"
    );

    let tail: Vec<DisplayMessage> = all_kinds_transcript()
        .display_messages
        .into_iter()
        .filter(|msg| {
            !msg.tool_data.as_ref().is_some_and(|tool| {
                super::tools_ui::canonical_tool_name(&tool.name) == "todo"
            })
        })
        .collect();
    assert_every_front_cut_point_resumes("unpinned_all_kinds", &tail, 72);
}

/// The same sweep with pinned todos disabled, so the skip path is not the only
/// zero-length segment in play.
#[test]
fn test_every_cut_point_resumes_without_pinned_todos() {
    let _env_lock = crate::storage::lock_test_env();
    let _pin = PinTodosGuard::disable();
    assert!(
        !crate::config::config().display.pin_todos,
        "this test must run with the todo card rendered inline"
    );

    let state = all_kinds_transcript();
    assert_every_cut_point_resumes("unpinned_all_kinds", &state, 72);

    // Same randomized sweep as the pinned test, so the inline card path is
    // covered over arbitrary neighbour combinations too.
    for seed in [7u64, 23] {
        let mut rng = Lcg::new(seed);
        let len = 8 + rng.pick(7);
        let state = random_transcript(&mut rng, len);
        let width = 40 + rng.pick(60) as u16;
        assert_every_cut_point_resumes(&format!("unpinned_rng_seed_{seed}"), &state, width);
    }
}
