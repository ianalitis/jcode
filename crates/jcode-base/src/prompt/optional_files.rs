//! Optional per-user and per-project prompt fragments (overlay, preferred tools).

use super::global_prompt_path;
use std::path::{Path, PathBuf};

fn load_optional_prompt_files(
    candidates: impl IntoIterator<Item = (Option<PathBuf>, &'static str)>,
) -> (Option<String>, usize) {
    let mut contents = Vec::new();
    let mut total_chars = 0;
    // With cwd = $HOME the project and global candidates resolve to the same
    // file; include it once so it is neither repeated in the prompt nor
    // counted twice against the budget (upstream #1092).
    let mut seen: Vec<PathBuf> = Vec::new();
    for (path, label) in candidates {
        let Some(path) = path else { continue };
        let Ok(canonical) = std::fs::canonicalize(&path) else {
            continue;
        };
        if seen.contains(&canonical) {
            continue;
        }
        if let Ok(content) = std::fs::read_to_string(&canonical) {
            total_chars += content.len();
            contents.push(format!("# {}\n\n{}", label, content.trim()));
            seen.push(canonical);
        }
    }
    (!contents.is_empty())
        .then(|| contents.join("\n\n"))
        .map_or((None, 0), |contents| (Some(contents), total_chars))
}

/// Load optional prompt overlay markdown from ~/.jcode/ and ./.jcode/
pub(super) fn load_prompt_overlay_files_from_dir(
    working_dir: Option<&Path>,
) -> (Option<String>, usize) {
    let project_dir = working_dir.unwrap_or(Path::new("."));
    load_optional_prompt_files([
        (
            Some(project_dir.join(".jcode/prompt-overlay.md")),
            "Project Prompt Overlay (.jcode/prompt-overlay.md)",
        ),
        (
            global_prompt_path(
                crate::storage::jcode_dir().map(|dir| dir.join("prompt-overlay.md")),
            ),
            "Global Prompt Overlay (~/.jcode/prompt-overlay.md)",
        ),
    ])
}

/// Load optional preferred-tool guidance from ~/.jcode/ and ./.jcode/
pub(super) fn load_preferred_tools_files_from_dir(
    working_dir: Option<&Path>,
) -> (Option<String>, usize) {
    let project_dir = working_dir.unwrap_or(Path::new("."));
    load_optional_prompt_files([
        (
            Some(project_dir.join(".jcode/preferred-tools.md")),
            "Project Preferred Tools (.jcode/preferred-tools.md)",
        ),
        (
            global_prompt_path(
                crate::storage::jcode_dir().map(|dir| dir.join("preferred-tools.md")),
            ),
            "Global Preferred Tools (~/.jcode/preferred-tools.md)",
        ),
    ])
}
