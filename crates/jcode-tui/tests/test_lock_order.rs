//! Deterministic regressions for the three cooperating env/render fixtures.
//! Parallel success alone cannot rule out ABBA. These concrete prefix checks
//! require outer render guards before direct or helper-owned env acquisition.
//! Later explicit drops still need review: this is not a Rust lifetime analyzer.

fn test_body<'a>(source: &'a str, name: &str) -> &'a str {
    source
        .split_once(&format!("fn {name}() {{"))
        .expect("regression target must exist")
        .1
        .split("#[test]")
        .next()
        .unwrap()
}

fn render_precedes_env(body: &str, render: &str, env: &str) -> bool {
    let mut lines = body
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with("//"));
    lines.next() == Some(render) && lines.next() == Some(env)
}

fn fixtures() -> [(&'static str, &'static str, &'static str); 3] {
    [
        (
            test_body(
                include_str!("../src/tui/app/tests/scroll_copy_02/part_02.rs"),
                "test_alt_shift_i_toggles_inline_images_and_persists",
            ),
            "let _render_lock = scroll_render_test_lock();",
            "let _env_guard = crate::storage::lock_test_env();",
        ),
        (
            test_body(
                include_str!("../src/tui/app/tests/scroll_copy_01/part_01.rs"),
                "test_chat_mouse_scroll_down_reaches_bottom_without_dead_zone",
            ),
            "let _lock = scroll_render_test_lock();",
            "let _env_lock = crate::storage::lock_test_env();",
        ),
        (
            test_body(
                include_str!("../src/tui/app/tests/smoothness_benchmark.rs"),
                "smoothness_benchmark_simulated_streaming_turn_stays_within_budget",
            ),
            "let _render_lock = scroll_render_test_lock();",
            "with_reasoning_current_home(|| {",
        ),
    ]
}

#[test]
fn inline_images_persistence_locks_render_before_env() {
    let (body, render, env) = fixtures()[0];
    assert!(
        render_precedes_env(body, render, env),
        "inline image persistence must hold outer render before env"
    );
}

#[test]
fn chat_mouse_scroll_locks_render_before_env() {
    let (body, render, env) = fixtures()[1];
    assert!(
        render_precedes_env(body, render, env),
        "scroll fixture must hold outer render before env"
    );
}

#[test]
fn smoothness_benchmark_locks_render_before_env() {
    let (body, render, env) = fixtures()[2];
    assert!(
        render_precedes_env(body, render, env),
        "benchmark must hold outer render before the env-taking home helper"
    );
}

fn replace_once(body: &str, old: &str, new: &str) -> String {
    assert_eq!(body.matches(old).count(), 1, "control preimage: {old}");
    body.replacen(old, new, 1)
}

#[test]
fn prefix_check_discriminates_actual_direct_and_helper_bodies() {
    for (body, render, env) in fixtures() {
        // Construct controls from each real body without changing source files.
        // Moving the one render binding out also supplies a positive C control
        // while its real-source assertion still rejects an inner callback guard.
        let without_render = replace_once(body, render, "");
        let render_first = format!("{render}\n{without_render}");
        assert!(render_precedes_env(&render_first, render, env));

        let reversed = replace_once(&without_render, env, &format!("{env}\n{render}"));
        assert!(!render_precedes_env(&reversed, render, env));
        assert!(!render_precedes_env(&without_render, render, env));
        let without_env = replace_once(&render_first, env, "");
        assert!(!render_precedes_env(&without_env, render, env));

        let short_render = format!("{{\n{render}\n}}\n{without_render}");
        assert!(!render_precedes_env(&short_render, render, env));
        let short_env = replace_once(&render_first, env, &format!("{{\n{env}\n}}"));
        assert!(!render_precedes_env(&short_env, render, env));
    }
}

#[test]
#[should_panic(expected = "regression target must exist")]
fn missing_named_target_fails() {
    test_body("fn other_test() {}", "missing_test");
}
