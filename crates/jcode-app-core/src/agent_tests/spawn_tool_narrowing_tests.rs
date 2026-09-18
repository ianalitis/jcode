//! Spawn-time tool allowlist narrowing (P1c of docs/HARNESS_LOOP_ARCHITECTURE.md).

use crate::agent::Agent;
use crate::config::ToolSelection;
use std::collections::HashSet;

fn set(names: &[&str]) -> HashSet<String> {
    names.iter().map(|s| s.to_string()).collect()
}

#[test]
fn none_keeps_configured_selection() {
    let configured = ToolSelection {
        allowed_tools: Some(set(&["read", "bash"])),
        disabled_tools: set(&["mcp"]),
    };
    let out = Agent::narrow_tool_selection(configured.clone(), None);
    assert_eq!(out, configured);
}

#[test]
fn spawn_allowlist_intersects_configured_allowlist() {
    let configured = ToolSelection {
        allowed_tools: Some(set(&["read", "bash", "edit"])),
        disabled_tools: HashSet::new(),
    };
    let spawn = vec!["read".to_string(), "write".to_string()];
    let out = Agent::narrow_tool_selection(configured, Some(&spawn));
    assert_eq!(
        out.allowed_tools,
        Some(set(&["read"])),
        "spawn cannot widen past config"
    );
}

#[test]
fn spawn_allowlist_applies_when_config_allows_everything() {
    let configured = ToolSelection {
        allowed_tools: None,
        disabled_tools: HashSet::new(),
    };
    let spawn = vec![" Read ".to_string(), "agentgrep".to_string()];
    let out = Agent::narrow_tool_selection(configured, Some(&spawn));
    assert_eq!(out.allowed_tools, Some(set(&["read", "agentgrep"])));
}

#[test]
fn empty_spawn_allowlist_yields_no_tools() {
    let configured = ToolSelection {
        allowed_tools: None,
        disabled_tools: HashSet::new(),
    };
    let out = Agent::narrow_tool_selection(configured, Some(&[]));
    assert_eq!(out.allowed_tools, Some(HashSet::new()));
}
