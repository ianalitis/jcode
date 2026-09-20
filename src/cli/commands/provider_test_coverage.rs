//! `jcode provider-test-coverage` reporting.

use anyhow::Result;
use std::io::IsTerminal;
use std::path::Path;

pub(crate) fn run_provider_test_coverage_command(
    provider_query: Option<String>,
    model_query: Option<String>,
    coverage_file: Option<&str>,
    coverage_limit: usize,
) -> Result<()> {
    let coverage_path = coverage_file.map(Path::new);
    let colorize = std::io::stdout().is_terminal()
        && std::env::var_os("NO_COLOR").is_none()
        && std::env::var_os("JCODE_NO_COLOR").is_none();
    let report = if let Some(provider) = provider_query {
        let model = model_query.unwrap_or_else(|| "*".to_string());
        crate::live_tests::format_provider_test_coverage_report(&provider, &model, coverage_path)
    } else {
        let (coverage, path) = crate::live_tests::load_coverage(coverage_path)?;
        let summary = crate::live_tests::strict_live_provider_model_coverage_summary(
            &coverage,
            path.display().to_string(),
        );
        crate::live_tests::format_strict_live_provider_model_coverage_summary(
            &summary,
            coverage_limit,
        )
    };
    if colorize {
        print!(
            "{}",
            crate::live_tests::colorize_provider_test_coverage_output(&report)
        );
    } else {
        print!("{report}");
    }
    Ok(())
}
