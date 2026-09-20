//! Path helpers for the cloud-sessions dashboard and its per-session views.

use std::path::{Path, PathBuf};

/// Directory that holds per-session viewer HTML files for a dashboard.
pub(super) fn dashboard_views_dir(dashboard_path: &Path) -> PathBuf {
    let stem = dashboard_path
        .file_stem()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| "dashboard".to_string());
    let parent = dashboard_path.parent().unwrap_or_else(|| Path::new("."));
    parent.join(format!("{stem}-views"))
}

/// Make a filesystem-safe filename component from a session id.
pub(super) fn sanitize_filename(value: &str) -> String {
    value
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.') {
                c
            } else {
                '_'
            }
        })
        .collect()
}

/// Build a link from the dashboard file to a viewer file, preferring a relative
/// path when both share a parent directory so the dashboard is portable.
pub(super) fn relative_link(dashboard_path: &Path, view_file: &Path) -> Option<String> {
    let base = dashboard_path.parent()?;
    // A view outside the dashboard's directory has no relative form; the
    // caller falls back to the absolute path.
    let Ok(rel) = view_file.strip_prefix(base) else {
        return None;
    };
    Some(rel.to_string_lossy().replace('\\', "/"))
}
