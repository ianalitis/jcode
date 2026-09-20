//! Score the deterministic baseline, or an external arm's outputs, against a
//! fixture set. No network, no models.
//!
//! ```text
//! s1-eval baseline <cases.json> <labels.json>
//! s1-eval external <arm-name> <outputs.json> <cases.json> <labels.json>
//! ```
//! External outputs are a JSON array of `Classification` records keyed by `id`
//! (extra fields are ignored), which is what the arm B and arm C scripts emit.

use jcode_s1_eval::{
    Classification, Classifier, DEFAULT_SKINS, DeterministicBaseline, IntakeCase, Label, score,
};
use std::collections::BTreeMap;

struct External {
    name: String,
    outputs: BTreeMap<String, Classification>,
}

impl Classifier for External {
    fn name(&self) -> &str {
        &self.name
    }
    fn classify(&self, case: &IntakeCase, _skins: &[&str]) -> Classification {
        self.outputs
            .get(&case.id)
            .cloned()
            .unwrap_or(Classification {
                abstain: true,
                ..Default::default()
            })
    }
}

fn read<T: serde::de::DeserializeOwned>(path: &str) -> Result<T, Box<dyn std::error::Error>> {
    let text = std::fs::read_to_string(path)?;
    Ok(serde_json::from_str(&text)?)
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();
    let sc = match args.get(1).map(String::as_str) {
        Some("baseline") if args.len() == 4 => {
            let cases: Vec<IntakeCase> = read(&args[2])?;
            let labels: Vec<Label> = read(&args[3])?;
            score(&DeterministicBaseline, &cases, &labels, DEFAULT_SKINS)
        }
        Some("external") if args.len() == 6 => {
            #[derive(serde::Deserialize)]
            struct Row {
                id: String,
                #[serde(flatten)]
                out: Classification,
            }
            // Accept a bare array or an object with an `outputs` array, which
            // is what the scratch arm scripts write.
            let raw: serde_json::Value = read(&args[3])?;
            let rows_value = raw.get("outputs").cloned().unwrap_or(raw);
            let rows: Vec<Row> =
                serde_json::from_value(rows_value).map_err(|e| format!("{}: {e}", args[3]))?;
            let cases: Vec<IntakeCase> = read(&args[4])?;
            let labels: Vec<Label> = read(&args[5])?;
            let arm = External {
                name: args[2].clone(),
                outputs: rows.into_iter().map(|r| (r.id, r.out)).collect(),
            };
            score(&arm, &cases, &labels, DEFAULT_SKINS)
        }
        _ => {
            eprintln!(
                "usage:\n  s1-eval baseline <cases.json> <labels.json>\n  s1-eval external <arm> <outputs.json> <cases.json> <labels.json>"
            );
            std::process::exit(2);
        }
    };
    let answerable = sc.cases - sc.abstain_expected;
    eprintln!(
        "{}: cases={} industry={}/{} task_kind={}/{} complexity={}/{} abstain={}/{} over_abstain={} skin_recall={}/{} invalid={} critical={}",
        sc.arm,
        sc.cases,
        sc.industry_correct,
        answerable,
        sc.task_kind_correct,
        answerable,
        sc.complexity_correct,
        answerable,
        sc.abstain_correct,
        sc.abstain_expected,
        sc.over_abstained,
        sc.skin_recall_hits,
        sc.skin_recall_denominator,
        sc.invalid_outputs,
        sc.critical_failures
    );
    println!("{}", serde_json::to_string_pretty(&sc)?);
    Ok(())
}
