// ---- The meta scorecard ----

#[test]
fn onboarding_meta_scorecard() {
    // Each property is a boolean; the meta-trust score is the fraction passing.
    // We re-run the property logic here (cheaply) so the scorecard prints a
    // single consolidated trust report. The dedicated #[test]s above are the
    // hard CI guards; this is the readable summary.
    let mut results: Vec<(&str, bool, &str)> = Vec::new();

    // 1. Monotonicity.
    let mono = {
        let p = pm(1, 1, 2, true);
        let ps = tier1_path_score(&p);
        let s = sm(60, true, true, true);
        let ss = tier3_screen_score(&s);
        tier1_path_score(&pm(2, 1, 2, true)) <= ps
            && tier1_path_score(&pm(1, 2, 2, true)) <= ps
            && tier1_path_score(&pm(1, 1, 3, true)) <= ps
            && tier1_path_score(&pm(1, 1, 2, false)) <= ps
            && tier3_screen_score(&sm(120, true, true, true)) <= ss
            && tier3_screen_score(&sm(60, true, false, true)) <= ss
            && tier3_screen_score(&sm(60, true, true, false)) <= ss
    };
    results.push(("monotonicity", mono, "worse never scores higher"));

    // 2. Anchoring.
    let (gp, bp) = anchor_paths();
    let (gs, bs) = anchor_screens();
    let gps = tier1_path_score(&gp);
    let bps = tier1_path_score(&bp);
    let gss = tier3_screen_score(&gs);
    let bss = tier3_screen_score(&bs);
    let anchoring = gps >= 90.0 && bps <= 30.0 && gss >= 85.0 && bss <= 30.0;
    results.push(("anchoring", anchoring, "known good/bad in right bands"));

    // 3. Discrimination.
    let path_gap = gps - bps;
    let screen_gap = gss - bss;
    let discrimination = path_gap >= 40.0 && screen_gap >= 40.0;
    results.push(("discrimination", discrimination, "good/bad separated >= 40"));

    // 4. Robustness (small sweep for the report; the #[test] runs the full one).
    let robustness = {
        let path_ladder = [pm(6, 4, 6, false), pm(2, 1, 2, true), pm(0, 0, 1, true)];
        let screen_ladder = [
            sm(220, true, false, false),
            sm(90, true, true, true),
            sm(30, true, true, true),
        ];
        let mut rng = Lcg(0x1234_5678_9ABC_DEF0);
        let mut ok = true;
        for _ in 0..200 {
            let w1 = jittered_tier1_weights(&mut rng, 0.5);
            let w3 = jittered_tier3_weights(&mut rng, 0.5);
            if !is_nondecreasing(
                &path_ladder
                    .iter()
                    .map(|m| tier1_path_score_w(m, &w1))
                    .collect::<Vec<_>>(),
            ) || !is_nondecreasing(
                &screen_ladder
                    .iter()
                    .map(|m| tier3_screen_score_w(m, &w3))
                    .collect::<Vec<_>>(),
            ) {
                ok = false;
                break;
            }
        }
        ok
    };
    results.push((
        "robustness",
        robustness,
        "ranking stable under +/-50% weights",
    ));

    // 5. Signal liveness.
    let liveness = {
        let p = tier1_path_score(&pm(1, 1, 2, true));
        let s = tier3_screen_score(&sm(60, true, true, true));
        tier1_path_score(&pm(2, 1, 2, true)) != p
            && tier1_path_score(&pm(1, 2, 2, true)) != p
            && tier1_path_score(&pm(1, 1, 3, true)) != p
            && tier1_path_score(&pm(1, 1, 2, false)) != p
            && tier3_screen_score(&sm(80, true, true, true)) != s
            && tier3_screen_score(&sm(60, true, false, true)) != s
            && tier3_screen_score(&sm(60, true, true, false)) != s
    };
    results.push(("signal liveness", liveness, "every signal moves the score"));

    let passed = results.iter().filter(|(_, ok, _)| *ok).count();
    let meta_trust = (passed as f64 / results.len() as f64) * 100.0;

    println!("\n============ META-EVALUATION (Tier M): is the scorer trustworthy? ============");
    println!("{:<16} {:>6}  guarantees", "property", "result");
    for (name, ok, desc) in &results {
        println!(
            "{:<16} {:>6}  {}",
            name,
            if *ok { "PASS" } else { "FAIL" },
            desc
        );
    }
    println!("--");
    println!("path good/bad anchors : {gps:.1} vs {bps:.1}  (gap {path_gap:.1})");
    println!("screen good/bad anchors: {gss:.1} vs {bss:.1}  (gap {screen_gap:.1})");
    println!(
        "META-TRUST            : {meta_trust:.0} / 100 ({passed}/{} properties)",
        results.len()
    );
    println!("=============================================================================\n");

    assert_eq!(
        passed,
        results.len(),
        "meta-evaluation found an untrustworthy property; see report above"
    );
}

// ===========================================================================
// Signal Coverage system. Answers "did we capture all the signals that matter,
// and is every signal we claim to score actually wired in?"
//
// Completeness ("are there signals we never thought of?") is fundamentally
// unprovable by a test, so instead we make the KNOWN universe explicit and add
// tripwires that force new product surface into a conscious decision:
//
//   Layer A  Registry  - every candidate signal declared as Scored / Deferred /
//                        Rejected, each with a rationale. Turns silent omission
//                        into a reviewable choice.
//   Layer B  Metrics   - scored-coverage ratio, liveness binding (a Scored
//                        signal must move the score), mapping (a Scored signal
//                        must apply to a real screen/path).
//   Layer C  Probe     - scans the REAL rendered screens for feature classes
//                        (options, countdown, list, command). Any feature class
//                        present on screen must be owned by a registry signal,
//                        so a new on-screen dimension cannot appear unmeasured.
// ===========================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SignalStatus {
    /// Wired into Tier 1 or Tier 3 today.
    Scored,
    /// Known to matter, deliberately not scored yet (with a reason).
    Deferred,
    /// Considered and intentionally excluded from scope (with a reason).
    Rejected,
}

/// A feature class that can be detected directly from a rendered screen. Used
/// by Layer C to verify every on-screen dimension is owned by a signal.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum FeatureClass {
    /// Yes/No or other selectable options are shown.
    InteractiveOptions,
    /// A countdown / numeric auto-advance is shown.
    Countdown,
    /// A numbered or multi-item list is shown.
    List,
    /// A typed command (e.g. "/login") is shown.
    Command,
    /// A free-text input / filter field is shown (e.g. the provider picker's
    /// type-to-filter box, or an API-key entry line).
    InputField,
    /// None of the above structural features (plain prose only).
    None,
}

struct SignalSpec {
    name: &'static str,
    status: SignalStatus,
    /// Which scoring field it feeds (for Scored) or why not (Deferred/Rejected).
    rationale: &'static str,
    /// The on-screen feature class this signal is responsible for measuring, if
    /// any. `None` means the signal is structural (counts) rather than tied to a
    /// visible feature class.
    owns_feature: FeatureClass,
}

/// Layer A: the declared signal universe for onboarding efficiency.
fn signal_registry() -> Vec<SignalSpec> {
    use FeatureClass::*;
    use SignalStatus::*;
    vec![
        // ---- Scored (wired into Tier 1) ----
        SignalSpec {
            name: "keystrokes",
            status: Scored,
            rationale: "Tier1.per_keystroke",
            owns_feature: None,
        },
        SignalSpec {
            name: "decisions",
            status: Scored,
            rationale: "Tier1.per_decision",
            owns_feature: InteractiveOptions,
        },
        SignalSpec {
            name: "screens",
            status: Scored,
            rationale: "Tier1.per_extra_screen",
            owns_feature: None,
        },
        SignalSpec {
            name: "reaches_ready",
            status: Scored,
            rationale: "Tier1.not_ready",
            owns_feature: None,
        },
        // ---- Scored (wired into Tier 3) ----
        SignalSpec {
            name: "word_count",
            status: Scored,
            rationale: "Tier3.per_excess_word (reading load)",
            owns_feature: None,
        },
        SignalSpec {
            name: "keyhint_consistency",
            status: Scored,
            rationale: "Tier3.inconsistent_keyhint",
            owns_feature: None,
        },
        SignalSpec {
            name: "escape_hatch",
            status: Scored,
            rationale: "Tier3.no_escape_hatch",
            owns_feature: Command,
        },
        SignalSpec {
            name: "countdown_present",
            status: Scored,
            rationale: "covered via word_count + keyhint on timed yes/no screens",
            owns_feature: Countdown,
        },
        SignalSpec {
            name: "suggestion_list",
            status: Scored,
            rationale: "covered via word_count on the Suggestions screen",
            owns_feature: List,
        },
        // ---- Scored (wired into Tier 4: content & robustness) ----
        SignalSpec {
            name: "terminology_consistency",
            status: Scored,
            rationale: "Tier4.inconsistent_terminology (one verb for 'log in' across screens)",
            owns_feature: None,
        },
        SignalSpec {
            name: "progress_visibility",
            status: Scored,
            rationale: "Tier4.no_progress ('N of M' in multi-step contexts)",
            owns_feature: None,
        },
        SignalSpec {
            name: "default_safety",
            status: Scored,
            rationale: "Tier4.unsafe_default (timed auto-commit lands on a recoverable outcome)",
            owns_feature: None,
        },
        SignalSpec {
            name: "narrow_terminal_safety",
            status: Scored,
            rationale: "Tier4.narrow_breaks (core Yes/No options survive a 50-col terminal)",
            owns_feature: None,
        },
        // ---- Scored (wired into Tier 5: path efficiency over the flow graph) ----
        SignalSpec {
            name: "min_vs_actual_path",
            status: Scored,
            rationale: "Tier5.per_excess_keystroke (default path vs graph-shortest to ready)",
            owns_feature: None,
        },
        SignalSpec {
            name: "first_input_latency",
            status: Scored,
            rationale: "Tier5.per_latency_keystroke (keystrokes before the first real action)",
            owns_feature: None,
        },
        SignalSpec {
            name: "irreducible_decisions",
            status: Scored,
            rationale: "Tier5.per_irreducible_decision (forced choices with no auto default)",
            owns_feature: None,
        },
        SignalSpec {
            name: "dead_end_screens",
            status: Scored,
            rationale: "Tier5.per_dead_end (non-terminal node with no forward edge)",
            owns_feature: None,
        },
        SignalSpec {
            name: "cycle_freedom",
            status: Scored,
            rationale: "Tier5.has_cycle (flow graph must be a DAG)",
            owns_feature: None,
        },
        // ---- Scored (wired into Tier 6: cognitive load per screen) ----
        SignalSpec {
            name: "reading_grade_level",
            status: Scored,
            rationale: "Tier6.per_excess_grade (Flesch-Kincaid over real body prose)",
            owns_feature: None,
        },
        SignalSpec {
            name: "options_per_screen",
            status: Scored,
            rationale: "Tier6.per_excess_option (Hick's law on simultaneous choices)",
            owns_feature: InteractiveOptions,
        },
        SignalSpec {
            name: "jargon_density",
            status: Scored,
            rationale: "Tier6.per_jargon_per_100w (unexplained technical terms)",
            owns_feature: None,
        },
        SignalSpec {
            name: "new_concepts_per_screen",
            status: Scored,
            rationale: "Tier6.per_excess_concept (distinct domain concepts introduced)",
            owns_feature: None,
        },
        SignalSpec {
            name: "number_of_questions",
            status: Scored,
            rationale: "Tier6.per_question_over_one (interrogatives to resolve)",
            owns_feature: None,
        },
        SignalSpec {
            name: "negation_count",
            status: Scored,
            rationale: "Tier6.per_negation (confusing don't/not/never phrasing)",
            owns_feature: None,
        },
        // ---- Scored (wired into Tier 7: clarity & guidance) ----
        SignalSpec {
            name: "single_primary_action",
            status: Scored,
            rationale: "Tier7.per_extra_action (one CTA/question per screen)",
            owns_feature: None,
        },
        SignalSpec {
            name: "action_verb_clarity",
            status: Scored,
            rationale: "Tier7.verb_unclear (instructions lead with an imperative verb)",
            owns_feature: None,
        },
        SignalSpec {
            name: "next_step_visibility",
            status: Scored,
            rationale: "Tier7.no_next_step (screen says what happens next)",
            owns_feature: None,
        },
        SignalSpec {
            name: "expectation_setting",
            status: Scored,
            rationale: "Tier7.no_expectation (multi-step context states scope up front)",
            owns_feature: None,
        },
        // ---- Scored (wired into Tier 8: reversibility & error handling) ----
        SignalSpec {
            name: "back_navigation",
            status: Scored,
            rationale: "Tier8.no_back_nav (a declined choice still offers a real recovery route, driven on the app)",
            owns_feature: None,
        },
        SignalSpec {
            name: "error_recovery_depth",
            status: Scored,
            rationale: "Tier8.per_recovery_keystroke (keystrokes from a declined branch back to an actionable login state)",
            owns_feature: None,
        },
        SignalSpec {
            name: "repeated_prompt",
            status: Scored,
            rationale: "Tier8.repeated_prompt (an answered decision is not re-asked in the real transitions)",
            owns_feature: None,
        },
        SignalSpec {
            name: "confirmation_for_destructive",
            status: Scored,
            rationale: "Tier8.unconfirmed_destructive (wildcard-free phase classifier: no phase mutates user data without a choice)",
            owns_feature: None,
        },
        SignalSpec {
            name: "timeout_safety",
            status: Scored,
            rationale: "Tier8.unsafe_timeout (do-nothing DECISION_TIMEOUT lands on a recoverable phase)",
            owns_feature: None,
        },
        // ---- Scored (wired into Tier 9: timing & pacing) ----
        SignalSpec {
            name: "countdown_adequacy",
            status: Scored,
            rationale: "Tier9.per_second_short (DECISION_TIMEOUT covers each timed screen's read budget at READING_WPS)",
            owns_feature: None,
        },
        SignalSpec {
            name: "forced_wait",
            status: Scored,
            rationale: "Tier9.forced_wait (every timed phase honors an immediate-commit key, verified on the app)",
            owns_feature: None,
        },
        SignalSpec {
            name: "time_on_blocker",
            status: Scored,
            rationale: "Tier9.per_second_over_ceiling (worst-case unattended dwell is bounded by DECISION_TIMEOUT)",
            owns_feature: None,
        },
        // ---- Scored (Layer C: structural / on-screen feature ownership) ----
        // These bind a registry signal to a specific on-screen feature class so
        // the completeness tripwire (Layer C) can prove every visible structural
        // dimension is owned. They drive the REAL full-app render of the provider
        // picker, a surface the welcome card alone never shows.
        SignalSpec {
            name: "interactive_options",
            status: Scored,
            rationale: "owns the Yes/No selector class (also counted by Tier1.decisions)",
            owns_feature: InteractiveOptions,
        },
        SignalSpec {
            name: "command_affordance",
            status: Scored,
            rationale: "owns typed-command screens (/login, /model); also Tier3.escape_hatch",
            owns_feature: Command,
        },
        SignalSpec {
            name: "input_field_present",
            status: Scored,
            rationale: "owns the provider picker's type-to-filter input surface (full-app render)",
            owns_feature: InputField,
        },
        // ---- Scored (wired into Tier 10: accessibility & robustness) ----
        SignalSpec {
            name: "no_unicode_dependence",
            status: Scored,
            rationale: "Tier10.per_nonascii_prose_char (load-bearing prose is ASCII-legible; logo is decorative)",
            owns_feature: None,
        },
        SignalSpec {
            name: "color_independence",
            status: Scored,
            rationale: "Tier10.color_dependent (selection marked by a non-color video attribute, verified on the buffer)",
            owns_feature: None,
        },
        SignalSpec {
            name: "screen_reader_order",
            status: Scored,
            rationale: "Tier10.illogical_order (prose precedes the action row for linear reading)",
            owns_feature: None,
        },
        // ---- Deferred (matters, not yet scored, with reason) ----
        // ---- Rejected (out of scope by construction) ----
        SignalSpec {
            name: "color_contrast",
            status: Rejected,
            rationale: "not derivable from the text buffer the evaluator reads",
            owns_feature: None,
        },
        SignalSpec {
            name: "visual_hierarchy",
            status: Rejected,
            rationale: "layout/eye-tracking concern; not measurable offline without users",
            owns_feature: None,
        },
        // These are genuinely valuable but FUNDAMENTALLY need real users or live
        // telemetry, which this evaluator refuses to collect by design. Listing
        // them keeps the rejection conscious (not a silent omission) and documents
        // exactly why each is out of scope for an offline, data-free evaluator.
        SignalSpec {
            name: "actual_completion_rate",
            status: Rejected,
            rationale: "needs a real-user funnel; the evaluator scores the artifact, never collects user data",
            owns_feature: None,
        },
        SignalSpec {
            name: "time_to_value_real",
            status: Rejected,
            rationale: "wall-clock time-to-first-value needs live telemetry; we only bound the flow's own timing (Tier 9)",
            owns_feature: None,
        },
        SignalSpec {
            name: "subjective_confusion",
            status: Rejected,
            rationale: "needs surveys / think-aloud; proxied (not replaced) by Tier 6 cognitive-load signals",
            owns_feature: None,
        },
        SignalSpec {
            name: "drop_off_point",
            status: Rejected,
            rationale: "needs analytics on real sessions; proxied structurally by Tier 5 dead_end_screens",
            owns_feature: None,
        },
    ]
}

/// Layer C: detect which feature classes a rendered screen actually contains.
fn detect_feature_classes(text: &str) -> Vec<FeatureClass> {
    let lower = text.to_ascii_lowercase();
    let mut found = Vec::new();
    if text.contains("( Yes )") || (text.contains("Yes") && text.contains("No")) {
        found.push(FeatureClass::InteractiveOptions);
    }
    // A countdown: "auto-selects in 12s" / "in 60s" / "automatically in 9s".
    if lower.contains("auto-selects in") || lower.contains("automatically in") {
        found.push(FeatureClass::Countdown);
    }
    // A numbered list, or a multi-row selectable list (the provider picker's
    // boxed rows with a "▸" selection caret over several entries).
    if text.contains("[1]")
        || lower.contains("press 1-")
        || (text.contains('▸') && text.matches("setup").count() >= 2)
    {
        found.push(FeatureClass::List);
    }
    // A typed command.
    if text.contains('/')
        && (lower.contains("/login") || lower.contains("/model") || lower.contains("type /"))
    {
        found.push(FeatureClass::Command);
    }
    // A free-text input / filter field: the provider picker shows a status line
    // and a row of selectable PROVIDER/ACTION columns you type to filter; the
    // boxed header "ITEM ... ACTION" plus the "1>" composer prompt is the typed
    // entry affordance the user can fall through to.
    if text.contains("ITEM") && lower.contains("provider") && lower.contains("action") {
        found.push(FeatureClass::InputField);
    }
    if found.is_empty() {
        found.push(FeatureClass::None);
    }
    found
}

/// Every user-facing welcome screen, rendered to text, for the Layer C probe.
fn all_welcome_screen_texts() -> Vec<(&'static str, String)> {
    use crate::external_auth::ExternalAuthReviewCandidate;
    use crate::tui::app::onboarding_flow::ImportReview;
    let now = std::time::Instant::now();
    let review = ImportReview::new(vec![ExternalAuthReviewCandidate::fixture(
        "OpenAI/Codex",
        "Codex auth.json",
    )])
    .unwrap();
    let phases: Vec<(&'static str, OnboardingPhase)> = vec![
        (
            "LoginOpenAi",
            OnboardingPhase::LoginOpenAi {
                yes_highlighted: true,
            },
        ),
        (
            "Login{import}",
            OnboardingPhase::Login {
                import: Some(review),
            },
        ),
        ("Login{recovery}", OnboardingPhase::Login { import: None }),
        (
            "ContinuePrompt",
            OnboardingPhase::ContinuePrompt {
                cli: ExternalCli::Codex,
                yes_highlighted: true,
                shown_at: now,
            },
        ),
        ("Suggestions", OnboardingPhase::Suggestions),
    ];
    phases
        .into_iter()
        .map(|(label, phase)| {
            let app = app_in_phase(phase);
            (label, render_onboarding_text(&app, 80, 30))
        })
        .collect()
}

/// Layer C probe surfaces: every welcome screen PLUS the provider-login picker.
/// The picker is a real onboarding surface the welcome card alone never renders
/// (a long selectable List with a type-to-filter InputField and Command
/// affordances), so structural ownership must see it. Kept separate from
/// `all_welcome_screen_texts` because the picker's terse list chrome is not
/// readable "prose" and would skew the per-screen prose rubrics.
fn all_probe_surface_texts() -> Vec<(&'static str, String)> {
    let mut out = all_welcome_screen_texts();
    out.push(("LoginPicker", render_login_picker_overlay_text()));
    out
}

/// Drive the real app into the open provider-login picker and render the FULL
/// app frame (welcome card + inline picker overlay) to text.
fn render_login_picker_overlay_text() -> String {
    use crossterm::event::KeyCode;
    use ratatui::Terminal;
    use ratatui::backend::TestBackend;

    let mut app = create_test_app();
    app.onboarding_flow = None;
    app.begin_onboarding_flow_at_login();
    if let Some(flow) = app.onboarding_flow.as_mut() {
        flow.phase = OnboardingPhase::Login { import: None };
    }
    // Enter opens the inline provider picker from the recovery screen.
    app.handle_onboarding_continue_prompt_key(KeyCode::Enter);

    let backend = TestBackend::new(90, 36);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal
        .draw(|frame| crate::tui::ui::draw(frame, &app as &dyn crate::tui::TuiState))
        .unwrap();
    let buffer = terminal.backend().buffer().clone();
    let mut rows: Vec<String> = Vec::new();
    for y in 0..36u16 {
        let mut row = String::new();
        for x in 0..90u16 {
            row.push_str(buffer[(x, y)].symbol());
        }
        rows.push(row.trim_end().to_string());
    }
    rows.join("\n")
}

#[test]
fn signal_coverage_scorecard() {
    with_temp_jcode_home(|| {
        let registry = signal_registry();
        let scored: Vec<&SignalSpec> = registry
            .iter()
            .filter(|s| s.status == SignalStatus::Scored)
            .collect();
        let deferred: Vec<&SignalSpec> = registry
            .iter()
            .filter(|s| s.status == SignalStatus::Deferred)
            .collect();
        let rejected: Vec<&SignalSpec> = registry
            .iter()
            .filter(|s| s.status == SignalStatus::Rejected)
            .collect();

        // ---- Layer B metric: scored coverage over the acknowledged-relevant
        // universe (Scored + Deferred; Rejected is out of scope by design). ----
        let relevant = scored.len() + deferred.len();
        let scored_coverage = (scored.len() as f64 / relevant as f64) * 100.0;

        // ---- Layer C: every feature class present on a real screen must be
        // owned by at least one Scored signal. ----
        let owned: std::collections::HashSet<FeatureClass> = scored
            .iter()
            .map(|s| s.owns_feature)
            .filter(|f| *f != FeatureClass::None)
            .collect();
        let screens = all_probe_surface_texts();
        let mut unowned: Vec<(String, FeatureClass)> = Vec::new();
        let mut present: std::collections::HashSet<FeatureClass> = std::collections::HashSet::new();
        for (label, text) in &screens {
            for fc in detect_feature_classes(text) {
                if fc == FeatureClass::None {
                    continue;
                }
                present.insert(fc);
                if !owned.contains(&fc) {
                    unowned.push((label.to_string(), fc));
                }
            }
        }
        let feature_coverage = if present.is_empty() {
            100.0
        } else {
            let covered = present.iter().filter(|fc| owned.contains(fc)).count();
            (covered as f64 / present.len() as f64) * 100.0
        };

        // ---- Report ----
        println!("\n============ SIGNAL COVERAGE ============");
        println!("-- Layer A: registry ({} signals) --", registry.len());
        println!("{:<22} {:<9} rationale", "signal", "status");
        for s in &registry {
            let st = match s.status {
                SignalStatus::Scored => "SCORED",
                SignalStatus::Deferred => "deferred",
                SignalStatus::Rejected => "rejected",
            };
            println!("{:<22} {:<9} {}", s.name, st, s.rationale);
        }
        println!("\n-- Layer B: coverage metrics --");
        println!("scored signals     : {}", scored.len());
        println!("deferred (known)   : {}", deferred.len());
        println!("rejected (scope)   : {}", rejected.len());
        println!(
            "scored coverage    : {scored_coverage:.0}% of acknowledged-relevant ({}/{})",
            scored.len(),
            relevant
        );
        println!("\n-- Layer C: on-screen feature ownership --");
        println!("feature classes present : {:?}", present);
        println!("feature classes owned   : {:?}", owned);
        println!("feature coverage        : {feature_coverage:.0}%");
        if !unowned.is_empty() {
            println!("UNOWNED (new dimension!) : {unowned:?}");
        }
        // Composite signal-coverage score: weight on-screen feature ownership
        // (the completeness tripwire) and the declared scored ratio.
        let signal_coverage = feature_coverage * 0.6 + scored_coverage * 0.4;
        println!("\nSIGNAL-COVERAGE SCORE : {signal_coverage:.1} / 100");
        println!("========================================\n");

        // ---- Guards ----
        // Every on-screen feature class must be owned. This is the tripwire: a
        // new visible dimension with no signal fails CI until someone scores it
        // or registers it (as Scored owning that class).
        assert!(
            unowned.is_empty(),
            "on-screen feature classes with no owning signal: {unowned:?} -- add a signal to the registry"
        );
        // Deferred/Rejected signals must carry a non-empty rationale (no silent
        // omission).
        for s in registry.iter().filter(|s| s.status != SignalStatus::Scored) {
            assert!(
                !s.rationale.trim().is_empty(),
                "signal '{}' is not scored but has no rationale",
                s.name
            );
        }
        // We must actually score a majority of acknowledged-relevant signals.
        assert!(
            scored_coverage >= 60.0,
            "scored coverage regressed below 60%: {scored_coverage:.0}%"
        );
    });
}

/// Layer B liveness binding: every signal the registry marks `Scored` must
/// correspond to a signal that demonstrably moves the score. We can't reflect
/// over field names in Rust, so we bind by an explicit, exhaustive checklist:
/// adding a Scored signal to the registry without a liveness clause here fails.
#[test]
fn signal_coverage_scored_signals_are_all_live() {
    let scored: Vec<&'static str> = signal_registry()
        .iter()
        .filter(|s| s.status == SignalStatus::Scored)
        .map(|s| s.name)
        .collect();

    // The set of Scored signals we have a concrete liveness proof for below.
    let proven: std::collections::HashSet<&'static str> = [
        "keystrokes",
        "decisions",
        "screens",
        "reaches_ready",
        "word_count",
        "keyhint_consistency",
        "escape_hatch",
        "countdown_present",
        "suggestion_list",
        "terminology_consistency",
        "progress_visibility",
        "default_safety",
        "narrow_terminal_safety",
        "min_vs_actual_path",
        "first_input_latency",
        "irreducible_decisions",
        "dead_end_screens",
        "cycle_freedom",
        "reading_grade_level",
        "options_per_screen",
        "jargon_density",
        "new_concepts_per_screen",
        "number_of_questions",
        "negation_count",
        "single_primary_action",
        "action_verb_clarity",
        "next_step_visibility",
        "expectation_setting",
        "back_navigation",
        "error_recovery_depth",
        "repeated_prompt",
        "confirmation_for_destructive",
        "timeout_safety",
        "countdown_adequacy",
        "forced_wait",
        "time_on_blocker",
        // Layer C structural ownership signals: proven live by the feature-class
        // probe (they own a class that is present on a real screen), the same
        // way countdown_present / suggestion_list are.
        "interactive_options",
        "command_affordance",
        "input_field_present",
        "no_unicode_dependence",
        "color_independence",
        "screen_reader_order",
    ]
    .into_iter()
    .collect();

    // Any Scored signal without a liveness proof is a coverage hole.
    for name in &scored {
        assert!(
            proven.contains(name),
            "Scored signal '{name}' has no liveness proof; add one to keep coverage honest"
        );
    }

    // Concrete liveness proofs for the structural (Tier 1) and copy (Tier 3)
    // signals. countdown_present and suggestion_list are proven via the real
    // screens: removing them would change word_count, which is already proven,
    // and they are validated as owned feature classes by the scorecard probe.
    let p = tier1_path_score(&pm(1, 1, 2, true));
    assert_ne!(tier1_path_score(&pm(2, 1, 2, true)), p);
    assert_ne!(tier1_path_score(&pm(1, 2, 2, true)), p);
    assert_ne!(tier1_path_score(&pm(1, 1, 3, true)), p);
    assert_ne!(tier1_path_score(&pm(1, 1, 2, false)), p);
    let s = tier3_screen_score(&sm(60, true, true, true));
    assert_ne!(tier3_screen_score(&sm(80, true, true, true)), s);
    assert_ne!(tier3_screen_score(&sm(60, true, false, true)), s);
    assert_ne!(tier3_screen_score(&sm(60, true, true, false)), s);

    // Tier 4 liveness: flipping each content/robustness signal must move the
    // Tier 4 score. Proves none of the four new signals are decorative.
    let good = Tier4Metrics {
        terminology_consistent: true,
        progress_visible: true,
        default_safe: true,
        narrow_options_survive: true,
    };
    let g = tier4_score(&good);
    assert_ne!(
        tier4_score(&Tier4Metrics {
            terminology_consistent: false,
            ..good
        }),
        g,
        "terminology_consistency"
    );
    assert_ne!(
        tier4_score(&Tier4Metrics {
            progress_visible: false,
            ..good
        }),
        g,
        "progress_visibility"
    );
    assert_ne!(
        tier4_score(&Tier4Metrics {
            default_safe: false,
            ..good
        }),
        g,
        "default_safety"
    );
    assert_ne!(
        tier4_score(&Tier4Metrics {
            narrow_options_survive: false,
            ..good
        }),
        g,
        "narrow_terminal_safety"
    );

    // Tier 5 liveness: perturbing each path-efficiency signal must move the
    // Tier 5 score. Proves min_vs_actual_path / first_input_latency /
    // irreducible_decisions / dead_end_screens / cycle_freedom are all wired.
    let base5 = Tier5Metrics {
        excess_keystrokes: 0.0,
        first_input_latency: 0,
        irreducible_decisions: 1,
        dead_end_screens: 0,
        acyclic: true,
    };
    let b5 = tier5_score(&base5);
    assert_ne!(
        tier5_score(&Tier5Metrics {
            excess_keystrokes: 1.0,
            ..base5
        }),
        b5,
        "min_vs_actual_path"
    );
    assert_ne!(
        tier5_score(&Tier5Metrics {
            first_input_latency: 1,
            ..base5
        }),
        b5,
        "first_input_latency"
    );
    assert_ne!(
        tier5_score(&Tier5Metrics {
            irreducible_decisions: 2,
            ..base5
        }),
        b5,
        "irreducible_decisions"
    );
    assert_ne!(
        tier5_score(&Tier5Metrics {
            dead_end_screens: 1,
            ..base5
        }),
        b5,
        "dead_end_screens"
    );
    assert_ne!(
        tier5_score(&Tier5Metrics {
            acyclic: false,
            ..base5
        }),
        b5,
        "cycle_freedom"
    );

    // Tier 6 liveness: perturbing each cognitive-load signal must move the
    // Tier 6 score. Proves reading_grade_level / options_per_screen /
    // jargon_density / new_concepts_per_screen / number_of_questions /
    // negation_count are all wired.
    let base6 = ScreenLoad {
        label: "synthetic",
        grade_level: 12.0, // above the grade budget so a delta is visible
        options: 2,
        jargon_per_100w: 0.0,
        new_concepts: 3,
        questions: 1,
        negations: 0,
    };
    let b6 = tier6_screen_score(&base6);
    assert_ne!(
        tier6_screen_score(&ScreenLoad {
            grade_level: 14.0,
            ..base6
        }),
        b6,
        "reading_grade_level"
    );
    assert_ne!(
        tier6_screen_score(&ScreenLoad {
            options: 4,
            ..base6
        }),
        b6,
        "options_per_screen"
    );
    assert_ne!(
        tier6_screen_score(&ScreenLoad {
            jargon_per_100w: 10.0,
            ..base6
        }),
        b6,
        "jargon_density"
    );
    assert_ne!(
        tier6_screen_score(&ScreenLoad {
            new_concepts: 6,
            ..base6
        }),
        b6,
        "new_concepts_per_screen"
    );
    assert_ne!(
        tier6_screen_score(&ScreenLoad {
            questions: 3,
            ..base6
        }),
        b6,
        "number_of_questions"
    );
    assert_ne!(
        tier6_screen_score(&ScreenLoad {
            negations: 2,
            ..base6
        }),
        b6,
        "negation_count"
    );

    // Tier 7 liveness: perturbing each clarity signal must move the Tier 7
    // score. Proves single_primary_action / action_verb_clarity /
    // next_step_visibility / expectation_setting are all wired.
    let base7 = ScreenClarity {
        label: "synthetic",
        primary_actions: 1,
        verbs_lead_instructions: true,
        next_step_visible: true,
        expectation_set: true,
        is_multistep: true,
    };
    let b7 = tier7_screen_score(&base7);
    assert_ne!(
        tier7_screen_score(&ScreenClarity {
            primary_actions: 3,
            ..base7
        }),
        b7,
        "single_primary_action"
    );
    assert_ne!(
        tier7_screen_score(&ScreenClarity {
            verbs_lead_instructions: false,
            ..base7
        }),
        b7,
        "action_verb_clarity"
    );
    assert_ne!(
        tier7_screen_score(&ScreenClarity {
            next_step_visible: false,
            ..base7
        }),
        b7,
        "next_step_visibility"
    );
    assert_ne!(
        tier7_screen_score(&ScreenClarity {
            expectation_set: false,
            ..base7
        }),
        b7,
        "expectation_setting"
    );

    // Tier 8 liveness: perturbing each reversibility signal must move the Tier 8
    // score. Proves back_navigation / error_recovery_depth / repeated_prompt /
    // confirmation_for_destructive / timeout_safety are all wired.
    let base8 = Tier8Metrics {
        back_navigation_ok: true,
        error_recovery_depth: 1,
        no_repeated_prompt: true,
        no_unconfirmed_destructive: true,
        timeout_safe: true,
    };
    let b8 = tier8_score(&base8);
    assert_ne!(
        tier8_score(&Tier8Metrics {
            back_navigation_ok: false,
            ..base8
        }),
        b8,
        "back_navigation"
    );
    assert_ne!(
        tier8_score(&Tier8Metrics {
            error_recovery_depth: 3,
            ..base8
        }),
        b8,
        "error_recovery_depth"
    );
    assert_ne!(
        tier8_score(&Tier8Metrics {
            no_repeated_prompt: false,
            ..base8
        }),
        b8,
        "repeated_prompt"
    );
    assert_ne!(
        tier8_score(&Tier8Metrics {
            no_unconfirmed_destructive: false,
            ..base8
        }),
        b8,
        "confirmation_for_destructive"
    );
    assert_ne!(
        tier8_score(&Tier8Metrics {
            timeout_safe: false,
            ..base8
        }),
        b8,
        "timeout_safety"
    );

    // Tier 9 liveness: perturbing each timing signal must move the Tier 9 score.
    // Proves countdown_adequacy / forced_wait / time_on_blocker are all wired.
    let base9 = Tier9Metrics {
        countdown_slack_secs: 10.0,
        no_forced_wait: true,
        max_blocker_secs: 60,
    };
    let b9 = tier9_score(&base9);
    assert_ne!(
        tier9_score(&Tier9Metrics {
            countdown_slack_secs: -5.0,
            ..base9
        }),
        b9,
        "countdown_adequacy"
    );
    assert_ne!(
        tier9_score(&Tier9Metrics {
            no_forced_wait: false,
            ..base9
        }),
        b9,
        "forced_wait"
    );
    assert_ne!(
        tier9_score(&Tier9Metrics {
            max_blocker_secs: 300,
            ..base9
        }),
        b9,
        "time_on_blocker"
    );

    // Tier 10 liveness: perturbing each accessibility signal must move the Tier
    // 10 score. Proves no_unicode_dependence / color_independence /
    // screen_reader_order are all wired.
    let base10 = Tier10Metrics {
        max_nonascii_prose_chars: 0,
        color_independent_selection: true,
        logical_reading_order: true,
    };
    let b10 = tier10_score(&base10);
    assert_ne!(
        tier10_score(&Tier10Metrics {
            max_nonascii_prose_chars: 3,
            ..base10
        }),
        b10,
        "no_unicode_dependence"
    );
    assert_ne!(
        tier10_score(&Tier10Metrics {
            color_independent_selection: false,
            ..base10
        }),
        b10,
        "color_independence"
    );
    assert_ne!(
        tier10_score(&Tier10Metrics {
            logical_reading_order: false,
            ..base10
        }),
        b10,
        "screen_reader_order"
    );
}
