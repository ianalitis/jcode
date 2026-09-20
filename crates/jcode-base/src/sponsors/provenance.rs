//! Inert compatibility API: this fork does not tag or meter discovered tools.
//!
//! There is no retained provenance, usage accumulator, timer, or upload path.
//! Enabling discovery in configuration cannot enable sponsor collection.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DiscoveredSetup {
    pub sponsor: String,
    pub command: String,
    pub args: Vec<String>,
}

pub fn record_discovered_setups(_setups: Vec<DiscoveredSetup>) {}

pub fn on_server_connected(_server_name: &str, _command: &str, _args: &[String]) -> Option<String> {
    None
}

pub fn on_tool_call(_server_name: &str, _is_error: bool) {}
pub fn is_tagged(_server_name: &str) -> bool {
    false
}
pub fn flush_now() {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn discovered_tools_are_not_tagged_or_collected() {
        let args = vec!["-y".into(), "example-mcp".into()];
        record_discovered_setups(vec![DiscoveredSetup {
            sponsor: "example".into(),
            command: "npx".into(),
            args: args.clone(),
        }]);
        assert_eq!(on_server_connected("example", "npx", &args), None);
        assert!(!is_tagged("example"));
        assert!(!is_tagged("private-server"));
        on_tool_call("example", false);
        on_tool_call("example", true);
        flush_now();
        assert!(!is_tagged("example"));
    }
}
