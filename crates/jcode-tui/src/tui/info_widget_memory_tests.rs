fn node(kind: &str, label: &str, degree: usize) -> GraphNode {
    GraphNode {
        id: format!("{}:{}", kind, label.replace(' ', "_")),
        label: label.to_string(),
        kind: kind.to_string(),
        is_memory: kind != "tag" && kind != "cluster",
        is_active: true,
        confidence: 0.9,
        degree,
    }
}

fn edge(source: usize, target: usize, kind: &str) -> GraphEdge {
    GraphEdge {
        source,
        target,
        kind: kind.to_string(),
    }
}

fn lines_text(lines: &[ratatui::text::Line<'_>]) -> String {
    lines
        .iter()
        .flat_map(|line| line.spans.iter())
        .map(|span| span.content.as_ref())
        .collect::<Vec<_>>()
        .join("\n")
}

#[test]
fn memory_widget_hides_sidecar_model_when_idle() {
    let info = MemoryInfo {
        total_count: 3,
        project_count: 2,
        global_count: 1,
        sidecar_available: true,
        sidecar_model: Some("openai · gpt-5.3-codex-spark".to_string()),
        ..Default::default()
    };
    let data = InfoWidgetData {
        memory_info: Some(info),
        ..Default::default()
    };

    let text = render_memory_widget(&data, Rect::new(0, 0, 40, 5))
        .all_lines()
        .iter()
        .flat_map(|line| line.spans.iter())
        .map(|span| span.content.as_ref())
        .collect::<Vec<_>>()
        .join("\n")
        .to_lowercase();

    assert!(text.contains("3 memories"));
    assert!(!text.contains("model:"));
    assert!(!text.contains("gpt-5.3"));
    assert!(text.contains("3 memories"));
}

#[test]
fn memory_widget_renders_current_cycle_activity() {
    let now = Instant::now();
    let mut pipeline = PipelineState::new();
    pipeline.search = StepStatus::Done;
    pipeline.verify = StepStatus::Running;
    pipeline.verify_progress = Some((1, 3));

    let data = InfoWidgetData {
        memory_info: Some(MemoryInfo {
            total_count: 7,
            project_count: 4,
            global_count: 3,
            sidecar_model: Some("openai · gpt-5.3-codex-spark".to_string()),
            activity: Some(MemoryActivity {
                state: MemoryState::SidecarChecking { count: 3 },
                state_since: now - Duration::from_secs(12),
                pipeline: Some(pipeline),
                recent_events: vec![
                    MemoryEvent {
                        kind: MemoryEventKind::MemoryInjected {
                            count: 2,
                            prompt_chars: 318,
                            age_ms: 44,
                            preview: "prefers terse answers".to_string(),
                            items: Vec::new(),
                        },
                        timestamp: now - Duration::from_secs(11),
                        detail: None,
                    },
                    MemoryEvent {
                        kind: MemoryEventKind::EmbeddingComplete {
                            latency_ms: 71,
                            hits: 9,
                        },
                        timestamp: now - Duration::from_secs(12),
                        detail: None,
                    },
                ],
            }),
            graph_nodes: vec![node("fact", "release build", 2), node("tag", "rust", 1)],
            graph_edges: vec![edge(0, 1, "has_tag")],
            ..Default::default()
        }),
        ..Default::default()
    };

    let text = render_memory_widget(&data, Rect::new(0, 0, 40, 8))
        .all_lines()
        .iter()
        .flat_map(|line| line.spans.iter())
        .map(|span| span.content.as_ref())
        .collect::<Vec<_>>()
        .join("\n")
        .to_lowercase();

    assert!(text.contains("7 memories"));
    assert!(text.contains("load memories"));
    assert!(text.contains("jev relevance"));
    assert!(text.contains("1/3"));
    assert!(text.contains("inject context"));
    assert!(text.contains("update memory"));
    assert!(text.contains("now:"));
    assert!(text.contains("jev relevance: 3"));
    assert!(!text.contains("model:"));
    assert!(!text.contains("gpt-5.3"));
    assert!(!text.contains("4 project"));
    assert!(!text.contains("3 global"));
}

#[test]
fn memory_widget_marks_completed_pipeline_even_when_state_is_idle() {
    let now = Instant::now();
    let mut pipeline = PipelineState::new();
    pipeline.search = StepStatus::Done;
    pipeline.verify = StepStatus::Done;
    pipeline.inject = StepStatus::Done;
    pipeline.maintain = StepStatus::Done;

    let data = InfoWidgetData {
        memory_info: Some(MemoryInfo {
            sidecar_model: Some("openai · gpt-5.3-codex-spark".to_string()),
            activity: Some(MemoryActivity {
                state: MemoryState::Idle,
                state_since: now - Duration::from_secs(4),
                pipeline: Some(pipeline),
                recent_events: vec![MemoryEvent {
                    kind: MemoryEventKind::MemoryInjected {
                        count: 1,
                        prompt_chars: 42,
                        age_ms: 12,
                        preview: "prefers terse answers".to_string(),
                        items: Vec::new(),
                    },
                    timestamp: now - Duration::from_secs(3),
                    detail: None,
                }],
            }),
            ..Default::default()
        }),
        ..Default::default()
    };

    let text = render_memory_widget(&data, Rect::new(0, 0, 40, 4))
        .all_lines()
        .iter()
        .flat_map(|line| line.spans.iter())
        .map(|span| span.content.as_ref())
        .collect::<Vec<_>>()
        .join("\n")
        .to_lowercase();

    assert!(text.contains("done"));
    assert!(text.contains("last:"));
}

#[test]
fn memory_widget_does_not_stay_done_after_idle_settles() {
    let now = Instant::now();
    let mut pipeline = PipelineState::new();
    pipeline.search = StepStatus::Done;
    pipeline.verify = StepStatus::Done;
    pipeline.inject = StepStatus::Done;
    pipeline.maintain = StepStatus::Done;

    let data = InfoWidgetData {
        memory_info: Some(MemoryInfo {
            total_count: 128,
            activity: Some(MemoryActivity {
                state: MemoryState::Idle,
                state_since: now - Duration::from_secs(12),
                pipeline: Some(pipeline),
                recent_events: vec![MemoryEvent {
                    kind: MemoryEventKind::MemoryInjected {
                        count: 1,
                        prompt_chars: 42,
                        age_ms: 12,
                        preview: "prefers terse answers".to_string(),
                        items: Vec::new(),
                    },
                    timestamp: now - Duration::from_secs(11),
                    detail: None,
                }],
            }),
            ..Default::default()
        }),
        ..Default::default()
    };

    let text = render_memory_widget(&data, Rect::new(0, 0, 50, 6))
        .all_lines()
        .iter()
        .flat_map(|line| line.spans.iter())
        .map(|span| span.content.as_ref())
        .collect::<Vec<_>>()
        .join("\n")
        .to_lowercase();

    assert!(text.contains("128 memories"), "{text}");
    assert!(!text.contains("done"), "{text}");
    assert!(!text.contains("idle"), "{text}");
    assert!(!text.contains("trace:"), "{text}");
}

#[test]
fn memory_widget_never_renders_uppercase_state_badges() {
    let data = InfoWidgetData {
        memory_info: Some(MemoryInfo {
            total_count: 128,
            activity: Some(MemoryActivity {
                state: MemoryState::SidecarChecking { count: 3 },
                state_since: Instant::now(),
                pipeline: None,
                recent_events: Vec::new(),
            }),
            ..Default::default()
        }),
        ..Default::default()
    };

    let text = render_memory_widget(&data, Rect::new(0, 0, 40, 8))
        .all_lines()
        .iter()
        .flat_map(|line| line.spans.iter())
        .map(|span| span.content.as_ref())
        .collect::<String>();

    assert!(text.contains("128 memories"), "{text}");
    for badge in [
        "IDLE", "SEARCH", "JEV", "READY", "INJECT", "SAVE", "UPDATE", "TOOL", "DONE", "FAILED",
        "DISABLED",
    ] {
        assert!(!text.contains(badge), "unexpected badge {badge}: {text}");
    }
}

#[test]
fn memory_widget_uses_distinct_trace_label_when_idle() {
    let now = Instant::now();
    let mut pipeline = PipelineState::new();
    pipeline.search = StepStatus::Done;
    pipeline.verify = StepStatus::Done;
    pipeline.inject = StepStatus::Done;
    pipeline.maintain = StepStatus::Done;

    let data = InfoWidgetData {
        memory_info: Some(MemoryInfo {
            sidecar_model: Some("openai · gpt-5.3-codex-spark".to_string()),
            activity: Some(MemoryActivity {
                state: MemoryState::Idle,
                state_since: now - Duration::from_secs(4),
                pipeline: Some(pipeline),
                recent_events: vec![MemoryEvent {
                    kind: MemoryEventKind::MemoryInjected {
                        count: 1,
                        prompt_chars: 42,
                        age_ms: 12,
                        preview: "prefers terse answers".to_string(),
                        items: Vec::new(),
                    },
                    timestamp: now - Duration::from_secs(3),
                    detail: None,
                }],
            }),
            ..Default::default()
        }),
        ..Default::default()
    };

    let text = render_memory_widget(&data, Rect::new(0, 0, 60, 8))
        .all_lines()
        .iter()
        .flat_map(|line| line.spans.iter())
        .map(|span| span.content.as_ref())
        .collect::<Vec<_>>()
        .join("\n")
        .to_lowercase();

    assert_eq!(text.matches("last:").count(), 1, "{text}");
    assert!(text.contains("trace:"), "{text}");
}

#[test]
fn memory_compact_does_not_show_model() {
    let lines = render_memory_compact(
        &MemoryInfo {
            sidecar_model: Some("openai · gpt-5.3-codex-spark".to_string()),
            ..Default::default()
        },
        30,
    );

    let text = lines
        .iter()
        .flat_map(|line| line.spans.iter())
        .map(|span| span.content.as_ref())
        .collect::<Vec<_>>()
        .join("\n")
        .to_lowercase();

    assert!(!text.contains("gpt-5.3"), "{text}");
    assert!(!text.contains("codex-spark"), "{text}");
}

#[test]
fn memory_compact_shows_memory_count_before_status() {
    let lines = render_memory_compact(
        &MemoryInfo {
            total_count: 128,
            activity: Some(MemoryActivity {
                state: MemoryState::Idle,
                state_since: Instant::now() - Duration::from_secs(8),
                pipeline: None,
                recent_events: Vec::new(),
            }),
            ..Default::default()
        },
        30,
    );

    let text = lines
        .iter()
        .flat_map(|line| line.spans.iter())
        .map(|span| span.content.as_ref())
        .collect::<Vec<_>>()
        .join("\n")
        .to_lowercase();

    assert!(text.contains("128 memories"), "{text}");
    assert!(!text.contains("idle"), "{text}");
    assert!(!text.contains(" · "), "{text}");
    assert!(!text.contains("memory ·"), "{text}");
}

#[test]
fn memory_widget_is_hidden_when_disabled() {
    let data = InfoWidgetData {
        memory_info: Some(MemoryInfo {
            total_count: 12,
            project_count: 8,
            global_count: 4,
            disabled: true,
            ..Default::default()
        }),
        ..Default::default()
    };

    assert!(render_memory_widget(&data, Rect::new(0, 0, 40, 5)).is_empty());
    assert!(render_memory_compact(data.memory_info.as_ref().unwrap(), 40).is_empty());
    assert!(!data.has_data_for(WidgetKind::MemoryActivity));
}

#[test]
fn memory_widget_shows_option_a_steps_without_pipeline_object() {
    let data = InfoWidgetData {
        memory_info: Some(MemoryInfo {
            sidecar_model: Some("openai · gpt-5.3-codex-spark".to_string()),
            activity: Some(MemoryActivity {
                state: MemoryState::SidecarChecking { count: 3 },
                state_since: Instant::now(),
                pipeline: None,
                recent_events: Vec::new(),
            }),
            ..Default::default()
        }),
        ..Default::default()
    };

    let text = render_memory_widget(&data, Rect::new(0, 0, 40, 8))
        .all_lines()
        .iter()
        .flat_map(|line| line.spans.iter())
        .map(|span| span.content.as_ref())
        .collect::<Vec<_>>()
        .join("\n")
        .to_lowercase();

    assert!(text.contains("load memories"), "{text}");
    assert!(text.contains("jev relevance"), "{text}");
    assert!(text.contains("inject context"), "{text}");
    assert!(text.contains("update memory"), "{text}");
    assert!(text.contains("jev relevance: 3"), "{text}");
}

#[test]
fn memory_activity_priority_is_elevated_while_processing() {
    let mut idle_data = InfoWidgetData {
        memory_info: Some(MemoryInfo {
            total_count: 2,
            activity: Some(MemoryActivity {
                state: MemoryState::Idle,
                state_since: Instant::now(),
                pipeline: None,
                recent_events: Vec::new(),
            }),
            ..Default::default()
        }),
        ..Default::default()
    };

    assert_eq!(
        idle_data.effective_priority(WidgetKind::MemoryActivity),
        WidgetKind::MemoryActivity.priority()
    );

    idle_data.memory_info.as_mut().unwrap().activity = Some(MemoryActivity {
        state: MemoryState::Embedding,
        state_since: Instant::now(),
        pipeline: None,
        recent_events: Vec::new(),
    });

    assert_eq!(idle_data.effective_priority(WidgetKind::MemoryActivity), 0);
}

#[test]
fn contextual_subgraph_prefers_memory_hub() {
    let mut nodes = vec![
        node("fact", "core build flow", 6),
        node("preference", "use cargo test", 4),
        node("tag", "rust", 5),
        node("tag", "testing", 3),
        node("fact", "docs in readme", 1),
    ];
    nodes[0].is_active = true;
    nodes[0].confidence = 0.95;

    let info = MemoryInfo {
        total_count: 5,
        graph_nodes: nodes,
        graph_edges: vec![
            edge(0, 1, "relates_to"),
            edge(0, 2, "has_tag"),
            edge(1, 3, "has_tag"),
            edge(4, 2, "has_tag"),
        ],
        ..Default::default()
    };

    let subgraph = super::select_contextual_subgraph(&info, 3, 6).expect("subgraph");
    assert_eq!(subgraph.nodes.len(), 3);
    assert!(
        subgraph
            .nodes
            .iter()
            .any(|n| n.label.contains("core build flow"))
    );
}
