//! Live end-to-end smoke of the no-tools Pi RPC adapter against a real pi.
//!
//! Run with an explicit provider/model, for example:
//! `cargo run -p jcode-executor-pi --example live_pi_smoke`
//!
//! This makes a real (metered) model call. It is not part of the test suite.

use jcode_attempt_types::{AttemptRecord, DataClass, Effort, LocalBudget, RouteClass};
use jcode_executor_pi::{PiConfig, run_pi_attempt};
use std::path::PathBuf;
use std::time::Duration;

fn main() {
    let model = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "deepseek/deepseek-v4-flash-0731".to_string());
    let provider = std::env::args()
        .nth(2)
        .unwrap_or_else(|| "openrouter".to_string());
    let binary = std::env::args().nth(3).unwrap_or_else(|| "pi".to_string());

    let attempt = AttemptRecord {
        task_id: "smoke".into(),
        attempt_id: "smoke/n1/a-1".into(),
        node_id: "n1".into(),
        provider: provider.clone(),
        model_exact: model.clone(),
        endpoint: "pi-rpc".into(),
        route_class: RouteClass::MeteredRemote,
        effort: Effort::Low,
        tool_allowlist: vec![],
        data_class: DataClass::Synthetic,
        router: None,
        deadline_secs: 120,
        budget: LocalBudget::default(),
        prompt_hash: "0".repeat(64),
        policy_version: "live-smoke".into(),
    }
    .freeze(chrono::Utc::now())
    .expect("attempt should freeze");

    let cfg = PiConfig {
        binary: PathBuf::from(&binary),
        provider: Some(provider.clone()),
        model: Some(model.clone()),
        cwd: std::env::temp_dir(),
    };

    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("runtime");
    let result = runtime
        .block_on(run_pi_attempt(
            &cfg,
            &attempt,
            "Reply with exactly: PONG",
            Duration::from_secs(120),
            None,
        ))
        .expect("pi attempt");

    println!("model:   {provider}/{model}");
    println!("outcome: {:?}", result.outcome);
    println!(
        "receipt: {}",
        serde_json::to_string_pretty(&result.receipt).expect("serialize receipt")
    );
}
