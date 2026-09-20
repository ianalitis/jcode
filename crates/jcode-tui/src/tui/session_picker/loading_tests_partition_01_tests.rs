#[test]
#[ignore = "developer benchmark: times real /resume loading phases"]
fn benchmark_real_resume_loading_phases() {
    invalidate_session_list_cache();

    let sessions_dir = storage::jcode_dir().expect("jcode dir").join("sessions");
    let scan_limit = session_scan_limit();
    let candidate_limit = session_candidate_window(scan_limit);

    let phase_start = std::time::Instant::now();
    let candidates = if sessions_dir.exists() {
        collect_recent_session_candidates(&sessions_dir, candidate_limit)
            .expect("collect recent session candidates")
    } else {
        Vec::new()
    };
    let collect_candidates_elapsed = phase_start.elapsed();

    let mut sessions = Vec::new();
    let mut skipped_empty = 0usize;
    let mut skipped_imported = 0usize;
    let mut summary_errors = 0usize;
    let phase_start = std::time::Instant::now();
    for stem in &candidates {
        if sessions.len() >= scan_limit {
            let saved = sessions_dir.join(format!("{stem}.json"));
            if !session_snapshot_or_journal_has_saved_metadata(&saved) {
                continue;
            }
        }
        if stem.starts_with("imported_cc_")
            || stem.starts_with("imported_codex_")
            || stem.starts_with("imported_pi_")
            || stem.starts_with("imported_opencode_")
        {
            skipped_imported += 1;
            continue;
        }

        let path = sessions_dir.join(format!("{stem}.json"));
        match load_session_summary(&path) {
            Ok(summary) if summary.messages.visible_message_count > 0 => {
                sessions.push((stem.clone(), summary));
            }
            Ok(_) => skipped_empty += 1,
            Err(_) => summary_errors += 1,
        }
    }
    let jcode_summary_elapsed = phase_start.elapsed();

    let phase_start = std::time::Instant::now();
    let claude = load_external_claude_code_sessions(scan_limit);
    let claude_elapsed = phase_start.elapsed();

    let phase_start = std::time::Instant::now();
    let codex = load_external_codex_sessions(scan_limit);
    let codex_elapsed = phase_start.elapsed();

    let phase_start = std::time::Instant::now();
    let pi = load_external_pi_sessions(scan_limit);
    let pi_elapsed = phase_start.elapsed();

    let phase_start = std::time::Instant::now();
    let opencode = load_external_opencode_sessions(scan_limit);
    let opencode_elapsed = phase_start.elapsed();

    let phase_start = std::time::Instant::now();
    let all_sessions = load_sessions().expect("load sessions");
    let load_sessions_elapsed = phase_start.elapsed();

    invalidate_session_list_cache();
    let phase_start = std::time::Instant::now();
    let (groups, orphans) = load_sessions_grouped().expect("load grouped sessions");
    let grouped_elapsed = phase_start.elapsed();

    let snapshot_count = std::fs::read_dir(&sessions_dir)
        .map(|entries| {
            entries
                .flatten()
                .filter(|entry| {
                    entry.file_name().to_str().is_some_and(|name| {
                        name.ends_with(".json") && !name.ends_with(".journal.json")
                    })
                })
                .count()
        })
        .unwrap_or_default();

    eprintln!(
        concat!(
            "real resume phases: scan_limit={} candidate_limit={} snapshot_count={} ",
            "candidate_count={} collect_candidates={}ms ",
            "jcode_summary={}ms jcode_loaded={} skipped_empty={} skipped_imported={} summary_errors={} ",
            "external_claude={}ms/{} external_codex={}ms/{} external_pi={}ms/{} external_opencode={}ms/{} ",
            "load_sessions={}ms/{} load_sessions_grouped={}ms groups={} orphans={}"
        ),
        scan_limit,
        candidate_limit,
        snapshot_count,
        candidates.len(),
        collect_candidates_elapsed.as_millis(),
        jcode_summary_elapsed.as_millis(),
        sessions.len(),
        skipped_empty,
        skipped_imported,
        summary_errors,
        claude_elapsed.as_millis(),
        claude.len(),
        codex_elapsed.as_millis(),
        codex.len(),
        pi_elapsed.as_millis(),
        pi.len(),
        opencode_elapsed.as_millis(),
        opencode.len(),
        load_sessions_elapsed.as_millis(),
        all_sessions.len(),
        grouped_elapsed.as_millis(),
        groups.len(),
        orphans.len(),
    );
}

#[test]
#[ignore = "developer benchmark: scans the real JCODE_HOME session directory"]
fn benchmark_real_resume_loading_reports_timings() {
    invalidate_session_list_cache();

    let load_start = std::time::Instant::now();
    let sessions = load_sessions().expect("load real sessions");
    let load_elapsed = load_start.elapsed();

    invalidate_session_list_cache();
    let grouped_start = std::time::Instant::now();
    let grouped = load_sessions_grouped().expect("load real grouped sessions");
    let grouped_elapsed = grouped_start.elapsed();
    let grouped_count = grouped
        .0
        .iter()
        .map(|group| group.sessions.len())
        .sum::<usize>()
        + grouped.1.len();

    eprintln!(
        "real resume bench: load_sessions={}ms count={} load_sessions_grouped={}ms grouped_count={} server_groups={} orphan_sessions={}",
        load_elapsed.as_millis(),
        sessions.len(),
        grouped_elapsed.as_millis(),
        grouped_count,
        grouped.0.len(),
        grouped.1.len()
    );
}

#[test]
fn benchmark_resume_loading_reports_timings() {
    let _env_lock = crate::storage::lock_test_env();
    let temp = tempfile::tempdir().expect("temp dir");
    let _home = EnvVarGuard::set_path("JCODE_HOME", temp.path());

    let sessions_dir = temp.path().join("sessions");
    std::fs::create_dir_all(&sessions_dir).expect("create sessions dir");

    for idx in 0..120 {
        let mut session = Session::create_with_id(
            format!("session_resume_bench_{idx:03}"),
            Some(format!("/tmp/resume-bench-{idx:03}")),
            Some(format!("Resume Bench {idx:03}")),
        );
        session.append_stored_message(crate::session::StoredMessage {
            id: format!("msg-{idx}-1"),
            role: crate::message::Role::User,
            content: vec![crate::message::ContentBlock::Text {
                text: format!("session {idx:03} says benchmark transcript token zebra-{idx:03}"),
                cache_control: None,
            }],
            display_role: None,
            timestamp: None,
            tool_duration_ms: None,
            token_usage: None,
        });
        session.append_stored_message(crate::session::StoredMessage {
            id: format!("msg-{idx}-2"),
            role: crate::message::Role::Assistant,
            content: vec![crate::message::ContentBlock::Text {
                text: "assistant reply for benchmark coverage".to_string(),
                cache_control: None,
            }],
            display_role: None,
            timestamp: None,
            tool_duration_ms: None,
            token_usage: None,
        });
        session.save().expect("save benchmark session");
    }

    let load_start = std::time::Instant::now();
    let sessions = load_sessions().expect("load sessions");
    let load_elapsed = load_start.elapsed();

    let group_start = std::time::Instant::now();
    let grouped = load_sessions_grouped().expect("load grouped sessions");
    let group_elapsed = group_start.elapsed();

    assert!(sessions.len() >= 100);
    assert!(!grouped.0.is_empty() || !grouped.1.is_empty());

    eprintln!(
        "resume bench: load_sessions={}ms load_sessions_grouped={}ms count={}",
        load_elapsed.as_millis(),
        group_elapsed.as_millis(),
        sessions.len()
    );
}

#[test]
fn onboarding_scoped_loader_returns_only_codex_sessions() {
    use crate::tui::app::onboarding_flow::ExternalCli;
    let _env_lock = crate::storage::lock_test_env();
    let temp = tempfile::tempdir().expect("temp dir");
    let _home = EnvVarGuard::set_path("JCODE_HOME", temp.path());

    // A Codex transcript that the onboarding picker should surface.
    let codex_dir = temp.path().join("external/.codex/sessions/2026/05/01");
    std::fs::create_dir_all(&codex_dir).expect("create codex dir");
    std::fs::write(
        codex_dir.join("rollout-2026-05-01T10-00-00-test.jsonl"),
        "{\"timestamp\":\"2026-05-01T10:00:00Z\",\"type\":\"session_meta\",\"payload\":{\"id\":\"codex-onboarding-test\",\"timestamp\":\"2026-05-01T09:59:00Z\",\"cwd\":\"/tmp/codex-onboard\"}}\n",
    )
    .expect("write codex transcript");

    // A jcode session that must NOT appear in the scoped Codex view (the whole
    // point of the scoped loader is to skip parsing these on onboarding).
    let mut jcode_session = Session::create_with_id(
        "session_onboarding_jcode_1780000000000".to_string(),
        Some("/tmp/jcode-onboard".to_string()),
        Some("Jcode Onboarding".to_string()),
    );
    jcode_session.append_stored_message(crate::session::StoredMessage {
        id: "msg-1".to_string(),
        role: crate::message::Role::User,
        content: vec![crate::message::ContentBlock::Text {
            text: "should not show in codex onboarding view".to_string(),
            cache_control: None,
        }],
        display_role: None,
        timestamp: None,
        tool_duration_ms: None,
        token_usage: None,
    });
    jcode_session.save().expect("save jcode session");

    let (groups, orphans) = load_external_cli_sessions_grouped(ExternalCli::Codex);
    assert!(groups.is_empty(), "scoped loader produces only orphans");
    assert!(
        orphans
            .iter()
            .any(|s| s.id == "codex:codex-onboarding-test"),
        "expected codex transcript in scoped onboarding load: {:?}",
        orphans.iter().map(|s| &s.id).collect::<Vec<_>>()
    );
    assert!(
        orphans
            .iter()
            .all(|s| matches!(s.resume_target, ResumeTarget::CodexSession { .. })),
        "scoped Codex load must not include jcode/other-CLI sessions"
    );
}

#[test]
fn parallel_fill_skips_many_recent_empty_sessions_to_reach_scan_limit() {
    let _env_lock = crate::storage::lock_test_env();
    let temp = tempfile::tempdir().expect("temp dir");
    let _home = EnvVarGuard::set_path("JCODE_HOME", temp.path());
    let _scan_limit = EnvVarGuard::set_str("JCODE_SESSION_PICKER_MAX_SESSIONS", "50");

    let sessions_dir = temp.path().join("sessions");
    std::fs::create_dir_all(&sessions_dir).expect("create sessions dir");

    let push_message = |session: &mut Session, text: &str| {
        session.append_stored_message(crate::session::StoredMessage {
            id: format!("msg-{text}"),
            role: crate::message::Role::User,
            content: vec![crate::message::ContentBlock::Text {
                text: text.to_string(),
                cache_control: None,
            }],
            display_role: None,
            timestamp: None,
            tool_duration_ms: None,
            token_usage: None,
        });
    };

    // Many recent but empty sessions (no visible messages) that the parallel
    // two-phase fill must skip while still collecting `scan_limit` real ones.
    for idx in 0..200 {
        let mut session = Session::create_with_id(
            format!("session_empty_{}", 1_790_000_000_000u64 + idx as u64),
            Some(format!("/tmp/empty-{idx:03}")),
            Some(format!("Empty {idx:03}")),
        );
        session.save().expect("save empty session");
    }
    // Older but non-empty sessions that should fill the list despite being less
    // recent than the empty stubs above.
    for idx in 0..60 {
        let mut session = Session::create_with_id(
            format!("session_full_{}", 1_780_000_000_000u64 + idx as u64),
            Some(format!("/tmp/full-{idx:03}")),
            Some(format!("Full {idx:03}")),
        );
        push_message(&mut session, &format!("real content {idx:03}"));
        session.save().expect("save full session");
    }

    invalidate_session_list_cache();
    let sessions = load_sessions().expect("load sessions");
    let visible: Vec<&SessionInfo> = sessions
        .iter()
        .filter(|s| s.id.starts_with("session_full_"))
        .collect();
    assert_eq!(
        visible.len(),
        50,
        "expected exactly scan_limit non-empty sessions, got {}",
        visible.len()
    );
    assert!(
        !sessions.iter().any(|s| s.id.starts_with("session_empty_")),
        "empty sessions must be filtered out of the loaded list"
    );
}

#[test]
fn hidden_debug_sessions_do_not_consume_default_resume_budget() {
    let _env_lock = crate::storage::lock_test_env();
    let temp = tempfile::tempdir().expect("temp dir");
    let _home = EnvVarGuard::set_path("JCODE_HOME", temp.path());
    let _scan_limit = EnvVarGuard::set_str("JCODE_SESSION_PICKER_MAX_SESSIONS", "50");

    let push_message = |session: &mut Session, text: &str| {
        session.append_stored_message(crate::session::StoredMessage {
            id: format!("msg-{text}"),
            role: crate::message::Role::User,
            content: vec![crate::message::ContentBlock::Text {
                text: text.to_string(),
                cache_control: None,
            }],
            display_role: None,
            timestamp: None,
            tool_duration_ms: None,
            token_usage: None,
        });
    };

    // Write ordinary sessions first so their filesystem mtimes are older than
    // the hidden debug burst below, matching the reported real-world ordering.
    for idx in 0..60 {
        let mut session = Session::create_with_id(
            format!("session_regular_{}", 1_780_000_000_000u64 + idx as u64),
            Some(format!("/tmp/regular-{idx:03}")),
            Some(format!("Regular {idx:03}")),
        );
        session.is_debug = false;
        session.is_canary = false;
        push_message(&mut session, &format!("regular content {idx:03}"));
        session.save().expect("save regular session");
    }

    // These newer self-dev/worker sessions are hidden by default. Previously the
    // loader stopped after the first 50, leaving no ordinary Jcode sessions for
    // the picker even though older resumable sessions existed.
    for idx in 0..75 {
        let mut session = Session::create_with_id(
            format!("session_debug_{}", 1_790_000_000_000u64 + idx as u64),
            Some(format!("/tmp/debug-{idx:03}")),
            Some(format!("Debug {idx:03}")),
        );
        session.is_debug = true;
        push_message(&mut session, &format!("debug content {idx:03}"));
        session.save().expect("save debug session");
    }

    invalidate_session_list_cache();
    let sessions = load_sessions().expect("load sessions");
    let regular_count = sessions.iter().filter(|session| !session.is_debug).count();
    let debug_count = sessions.iter().filter(|session| session.is_debug).count();

    assert_eq!(
        regular_count, 50,
        "ordinary sessions should fill the visible budget"
    );
    assert_eq!(
        debug_count, 50,
        "debug sessions should retain their own bounded budget"
    );
}

#[test]
fn session_matches_picker_query_requires_all_tokens_order_independent() {
    let _env_lock = crate::storage::lock_test_env();
    let temp = tempfile::tempdir().expect("temp dir");
    let _home = EnvVarGuard::set_path("JCODE_HOME", temp.path());

    let mut session = Session::create_with_id(
        "session_token_match".to_string(),
        Some("/tmp/token-match".to_string()),
        Some("Token Match".to_string()),
    );
    session.append_stored_message(crate::session::StoredMessage {
        id: "msg1".to_string(),
        role: crate::message::Role::User,
        content: vec![crate::message::ContentBlock::Text {
            text: "please deploy the production api gateway now".to_string(),
            cache_control: None,
        }],
        display_role: None,
        timestamp: None,
        tool_duration_ms: None,
        token_usage: None,
    });
    session.save().expect("save session");

    let sessions = load_sessions().expect("load sessions");
    let loaded = sessions
        .iter()
        .find(|candidate| candidate.id == "session_token_match")
        .expect("session present");

    // All tokens present, any order -> match (the old contiguous-substring matcher
    // would have failed on reordered / non-adjacent words).
    assert!(session_matches_picker_query(loaded, "api deploy"));
    assert!(session_matches_picker_query(loaded, "deploy api"));
    assert!(session_matches_picker_query(loaded, "  DEPLOY   Gateway  "));
    // A token that doesn't appear anywhere -> no match, even if others do.
    assert!(!session_matches_picker_query(loaded, "deploy staging"));
    // Empty query matches everything.
    assert!(session_matches_picker_query(loaded, "   "));
}

/// Regression tests for issue #674: the picker must be able to list only
/// jcode's own sessions, and toggling that must not be masked by either cache.
mod external_session_opt_out {
    use super::super::{GroupedSessionListDiskCache, session_list_disk_cache_is_usable};
    use std::path::{Path, PathBuf};

    fn disk_cache(dir: &Path, external_sessions: bool) -> GroupedSessionListDiskCache {
        GroupedSessionListDiskCache {
            version: super::super::SESSION_LIST_DISK_CACHE_VERSION,
            generated_at: chrono::Utc::now(),
            sessions_dir: dir.to_path_buf(),
            scan_limit: 50,
            include_old_saved_sessions: super::super::include_old_saved_sessions_on_initial_load(),
            external_sessions,
            server_groups: Vec::new(),
            orphan_sessions: Vec::new(),
        }
    }

    #[test]
    fn disk_cache_written_with_externals_is_rejected_after_opting_out() {
        let dir = PathBuf::from("/tmp/jcode-test-sessions");
        let cache = disk_cache(&dir, true);

        assert!(
            session_list_disk_cache_is_usable(&cache, &dir, 50, true),
            "same setting -> reusable"
        );
        assert!(
            !session_list_disk_cache_is_usable(&cache, &dir, 50, false),
            "opting out must not be served a cache that still contains other CLIs' sessions"
        );
    }

    #[test]
    fn disk_cache_written_without_externals_is_rejected_after_opting_back_in() {
        let dir = PathBuf::from("/tmp/jcode-test-sessions");
        let cache = disk_cache(&dir, false);

        assert!(session_list_disk_cache_is_usable(&cache, &dir, 50, false));
        assert!(
            !session_list_disk_cache_is_usable(&cache, &dir, 50, true),
            "opting back in must trigger a rescan"
        );
    }

    /// An older cache file has no `external_sessions` field; it was written
    /// with externals included, so it must deserialize that way.
    #[test]
    fn legacy_disk_cache_without_the_field_defaults_to_externals_included() {
        let json = serde_json::json!({
            "version": super::super::SESSION_LIST_DISK_CACHE_VERSION,
            "generated_at": chrono::Utc::now(),
            "sessions_dir": "/tmp/jcode-test-sessions",
            "scan_limit": 50,
            "include_old_saved_sessions": false,
            "server_groups": [],
            "orphan_sessions": []
        });
        let cache: GroupedSessionListDiskCache =
            serde_json::from_value(json).expect("legacy cache must still parse");
        assert!(cache.external_sessions);
    }

    /// The config default must keep today's behavior (externals shown).
    #[test]
    fn external_sessions_default_is_on() {
        assert!(jcode_config_types::DisplayConfig::default().external_sessions);
    }
}
