//! Produce the W3 footprint receipt.
//!
//! Runs the real arm over the bundled dev set in one child and reports what was
//! measured: the child's own peak RSS, the load and batch wall times, the
//! scorecard, and whether each acceptance item held. Nothing here is inferred
//! from the model's size on disk.
//!
//! ```sh
//! JCODE_LAYA_PYTHON=~/.jcode/local-arms/laya/venv/bin/python \
//! JCODE_LAYA_MODEL=~/.cache/huggingface/hub/models--convaiinnovations--laya/snapshots/<rev> \
//!   cargo run -p jcode-s1-laya-runtime --bin laya_footprint -- --out receipt.json
//! ```

use jcode_s1_eval::bundled_decision_fixtures;
use jcode_s1_laya_runtime::{LayaArmConfig, run_batch};
use serde_json::json;
use std::path::PathBuf;

fn main() -> std::process::ExitCode {
    let mut out: Option<PathBuf> = None;
    let mut arguments = std::env::args().skip(1);
    while let Some(argument) = arguments.next() {
        match argument.as_str() {
            "--out" => out = arguments.next().map(PathBuf::from),
            "--help" | "-h" => {
                println!("laya_footprint [--out <path>]");
                return std::process::ExitCode::SUCCESS;
            }
            other => {
                eprintln!("unknown argument: {other}");
                return std::process::ExitCode::from(2);
            }
        }
    }

    let config = LayaArmConfig::from_env();
    let fixtures = match bundled_decision_fixtures() {
        Ok(fixtures) => fixtures,
        Err(error) => {
            eprintln!("the bundled dev set did not parse: {error}");
            return std::process::ExitCode::from(2);
        }
    };

    // Recorded as absent rather than as zero if the clock is before the epoch,
    // so a receipt never carries a fabricated timestamp.
    let generated_unix_secs =
        match std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH) {
            Ok(elapsed) => Some(elapsed.as_secs()),
            Err(_) => None,
        };

    let report = match run_batch(&config, &fixtures) {
        Ok(report) => report,
        Err(error) => {
            eprintln!("the batch failed closed: {error}");
            return std::process::ExitCode::FAILURE;
        }
    };

    // The summary is the measurement. Without it there is no receipt to write,
    // so this is a failure rather than a zeroed report.
    let Some(summary) = report.summary.clone() else {
        eprintln!("the child did not summarise; the receipt would be incomplete");
        return std::process::ExitCode::FAILURE;
    };
    let under_ceiling = summary.peak_rss_bytes <= config.max_rss_bytes;

    let receipt = json!({
        "generated_unix_secs": generated_unix_secs,
        "arm": "laya-local",
        "config": {
            "python": config.python.display().to_string(),
            "script": config.script.display().to_string(),
            "model": config.model,
            "subfolder": config.subfolder,
            "device": config.device,
            "max_rss_bytes": config.max_rss_bytes,
            "request_timeout_ms": config.request_timeout.as_millis() as u64,
        },
        "load": report.ready.as_ref().map(|ready| json!({
            "device": ready.device,
            "model": ready.model,
            "load_ms": ready.load_ms,
        })),
        "summary": {
            "requests": summary.requests,
            "errors": summary.errors,
            "load_ms": summary.load_ms,
            "batch_wall_ms": summary.wall_ms,
            "input_tokens": summary.input_tokens,
            "peak_rss_bytes": summary.peak_rss_bytes,
            "peak_rss_mb": summary.peak_rss_bytes as f64 / (1024.0 * 1024.0),
        },
        "scorecard": report.scorecard,
        "spawns": report.spawns,
        "rejected_by_contract": report.rejected,
        "acceptance": {
            "invalid_is_zero": report.scorecard.invalid == 0,
            "critical_is_zero": report.scorecard.critical == 0,
            "rejected_is_zero": report.rejected == 0,
            "one_child_per_batch": report.spawns == 1,
            "peak_rss_measured": summary.peak_rss_bytes > 0,
            "peak_rss_under_ceiling": under_ceiling,
            "child_errors_zero": summary.errors == 0,
        },
        "child_stderr": report.diagnostics,
    });

    let rendered = match serde_json::to_string_pretty(&receipt) {
        Ok(rendered) => rendered,
        Err(error) => {
            eprintln!("could not render the receipt: {error}");
            return std::process::ExitCode::FAILURE;
        }
    };
    println!("{rendered}");

    if let Some(path) = out {
        if let Some(parent) = path.parent()
            && !parent.as_os_str().is_empty()
            && let Err(error) = std::fs::create_dir_all(parent)
        {
            eprintln!("could not create {}: {error}", parent.display());
            return std::process::ExitCode::FAILURE;
        }
        if let Err(error) = std::fs::write(&path, rendered + "\n") {
            eprintln!("could not write {}: {error}", path.display());
            return std::process::ExitCode::FAILURE;
        }
        eprintln!("receipt written to {}", path.display());
    }

    let all_held = report.scorecard.invalid == 0
        && report.scorecard.critical == 0
        && report.rejected == 0
        && report.spawns == 1
        && summary.errors == 0
        && under_ceiling;
    if all_held {
        std::process::ExitCode::SUCCESS
    } else {
        std::process::ExitCode::FAILURE
    }
}
