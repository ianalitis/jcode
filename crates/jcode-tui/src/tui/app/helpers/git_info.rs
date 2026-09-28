//! Stale-while-revalidate cache for the git status widget.

use super::backdated_now;
use crate::tui::info_widget::GitInfo;
use std::sync::Mutex;
use std::time::Duration;

/// Stale-while-revalidate cache for the git status widget. Module-level so the
/// app can force a refresh the moment it mutates the repo (commit, shell, file
/// edits) instead of waiting out the TTL with a stale branch/dirty count.
type GitInfoCacheEntry = (std::time::Instant, Option<GitInfo>, bool);
static GIT_INFO_CACHE: Mutex<Option<GitInfoCacheEntry>> = Mutex::new(None);

/// Force the git-status widget cache to refetch on its next read.
///
/// Call this right after the app changes the working tree or HEAD (commits,
/// shell commands, file edits) so the info widget reflects the new repo state
/// immediately rather than after the 5s TTL. Stale-while-revalidate still
/// applies: the next read returns the last value and kicks a background refresh.
pub(crate) fn invalidate_git_info_cache() {
    if let Ok(mut guard) = GIT_INFO_CACHE.lock()
        && let Some((ts, _cached, refreshing)) = guard.as_mut()
    {
        // Backdate the timestamp past the TTL so the next `gather_git_info`
        // treats the entry as expired and spawns a refresh, while still
        // returning the last-known value (no flicker to empty).
        *ts = backdated_now(Duration::from_secs(3600));
        *refreshing = false;
    }
}

/// Pin the git-status widget to a fixed value for deterministic renders.
///
/// Full-frame artifact generators (onboarding screenshots) would otherwise
/// capture the live ahead/behind/dirty counts of whatever repo the generator
/// happens to run in. Marking the entry as `refreshing` keeps the TTL path
/// from spawning a background probe that overwrites the seed mid-render.
#[cfg(test)]
pub(crate) fn seed_git_info_cache_for_tests(info: Option<GitInfo>) {
    if let Ok(mut guard) = GIT_INFO_CACHE.lock() {
        *guard = Some((std::time::Instant::now(), info, true));
    }
}

/// Tests never probe the live repository. The probe runs on a background
/// thread and writes its answer into this process-global cache, so the
/// first test to call this gets `None` while every later test in the same
/// binary silently inherits the developer's real branch and dirty counts.
/// That made frame assertions depend on how many tests ran before them and
/// on whether the checkout happened to be clean.
///
/// Tests that want git data seed it explicitly with
/// `seed_git_info_cache_for_tests`, which marks the entry `refreshing`.
#[cfg(test)]
pub(crate) fn gather_git_info() -> Option<GitInfo> {
    if crate::tui::is_ssh_remote() {
        return None;
    }
    GIT_INFO_CACHE
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .as_ref()
        .and_then(|(_, cached, _)| cached.clone())
}

#[cfg(not(test))]
pub(crate) fn gather_git_info() -> Option<GitInfo> {
    if crate::tui::is_ssh_remote() {
        return None;
    }
    use std::time::Instant;

    const TTL: Duration = Duration::from_secs(5);

    if let Ok(mut guard) = GIT_INFO_CACHE.lock() {
        if let Some((ts, cached, refreshing)) = guard.as_mut() {
            if ts.elapsed() < TTL {
                return cached.clone();
            }
            if *refreshing {
                return cached.clone();
            }
            let stale = cached.clone();
            *refreshing = true;
            std::thread::spawn(|| {
                let result = gather_git_info_inner();
                if let Ok(mut guard) = GIT_INFO_CACHE.lock() {
                    *guard = Some((Instant::now(), result, false));
                }
            });
            return stale;
        }

        *guard = Some((backdated_now(TTL + Duration::from_secs(1)), None, true));
        std::thread::spawn(|| {
            let result = gather_git_info_inner();
            if let Ok(mut guard) = GIT_INFO_CACHE.lock() {
                *guard = Some((Instant::now(), result, false));
            }
        });
    }
    None
}

#[cfg(not(test))]
fn gather_git_info_inner() -> Option<GitInfo> {
    gather_git_info_in(None)
}

/// Git status for `dir` (or the process working directory when `None`).
pub(crate) fn gather_git_info_in(dir: Option<&std::path::Path>) -> Option<GitInfo> {
    let git = || {
        let mut cmd = std::process::Command::new("git");
        if let Some(dir) = dir {
            cmd.current_dir(dir);
        }
        cmd
    };

    let in_repo = git()
        .args(["rev-parse", "--is-inside-work-tree"])
        .output()
        .ok()
        .map(|o| o.status.success())
        .unwrap_or(false);

    if !in_repo {
        return None;
    }

    let branch = git()
        .args(["branch", "--show-current"])
        .output()
        .ok()
        .and_then(|o| {
            if o.status.success() {
                let b = String::from_utf8_lossy(&o.stdout).trim().to_string();
                if b.is_empty() { None } else { Some(b) }
            } else {
                None
            }
        })
        .unwrap_or_else(|| "HEAD".to_string());

    let mut modified = 0;
    let mut staged = 0;
    let mut untracked = 0;
    let mut all_files: Vec<crate::tui::info_widget::DirtyFile> = Vec::new();

    let repo_root = git()
        .args(["rev-parse", "--show-toplevel"])
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| std::path::PathBuf::from(String::from_utf8_lossy(&o.stdout).trim()));

    if let Ok(output) = git()
        .args(["status", "--porcelain", "--untracked-files=all"])
        .output()
        && output.status.success()
    {
        let status = String::from_utf8_lossy(&output.stdout);
        for line in status.lines() {
            if line.len() < 3 {
                continue;
            }
            let index_status = line.as_bytes()[0];
            let worktree_status = line.as_bytes()[1];
            let file_path = line[3..].to_string();

            if index_status == b'?' {
                untracked += 1;
            } else {
                if index_status != b' ' && index_status != b'?' {
                    staged += 1;
                }
                if worktree_status != b' ' && worktree_status != b'?' {
                    modified += 1;
                }
            }

            all_files.push(crate::tui::info_widget::DirtyFile::new(
                porcelain_status_letter(index_status, worktree_status),
                file_path,
            ));
        }
    }

    // Line counts: tracked files from one numstat against HEAD (staged plus
    // unstaged), untracked files by counting their lines.
    let numstat = git()
        .args(["diff", "--numstat", "HEAD"])
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| parse_numstat(&String::from_utf8_lossy(&o.stdout)))
        .unwrap_or_default();
    let mut added_total = 0usize;
    let mut removed_total = 0usize;
    for file in &mut all_files {
        let key = file
            .path
            .rsplit(" -> ")
            .next()
            .unwrap_or(&file.path)
            .trim_matches('"');
        let abs = repo_root.as_ref().map(|root| root.join(key));
        if file.status == '?' {
            file.added = abs.as_deref().and_then(count_text_lines);
            file.removed = file.added.map(|_| 0);
        } else if let Some(&(a, r)) = numstat.get(key) {
            file.added = a;
            file.removed = r;
        }
        added_total += file.added.unwrap_or(0);
        removed_total += file.removed.unwrap_or(0);
        file.modified_at = abs
            .as_deref()
            .and_then(|p| std::fs::symlink_metadata(p).ok())
            .and_then(|m| m.modified().ok());
    }
    // Newest first, so the file being worked on stays visible under the cap.
    // Deleted files have no mtime and sort last.
    all_files.sort_by(|a, b| b.modified_at.cmp(&a.modified_at));
    let dirty_total = all_files.len();
    all_files.truncate(10);
    let dirty_files = all_files;

    let (ahead, behind) = git()
        .args(["rev-list", "--left-right", "--count", "HEAD...@{upstream}"])
        .output()
        .ok()
        .and_then(|o| {
            if o.status.success() {
                let text = String::from_utf8_lossy(&o.stdout).trim().to_string();
                let parts: Vec<&str> = text.split('\t').collect();
                if parts.len() == 2 {
                    let a = parts[0].parse::<usize>().unwrap_or(0);
                    let b = parts[1].parse::<usize>().unwrap_or(0);
                    Some((a, b))
                } else {
                    None
                }
            } else {
                None
            }
        })
        .unwrap_or((0, 0));

    Some(GitInfo {
        branch,
        modified,
        staged,
        untracked,
        ahead,
        behind,
        dirty_files,
        dirty_total,
        added_total,
        removed_total,
        repo_root,
    })
}

/// Parse `git diff --numstat` into path -> (added, removed). Binary files
/// report `-` and map to `None` counts.
pub(crate) fn parse_numstat(
    text: &str,
) -> std::collections::HashMap<String, (Option<usize>, Option<usize>)> {
    let mut out = std::collections::HashMap::new();
    for line in text.lines() {
        let mut parts = line.splitn(3, '\t');
        let (Some(a), Some(r), Some(path)) = (parts.next(), parts.next(), parts.next()) else {
            continue;
        };
        // Renames: `old => new` or `dir/{old => new}/file`.
        let path = if let (Some(open), Some(close)) = (path.find('{'), path.find('}')) {
            let inner = &path[open + 1..close];
            let new = inner.rsplit(" => ").next().unwrap_or(inner);
            format!("{}{}{}", &path[..open], new, &path[close + 1..]).replace("//", "/")
        } else {
            path.rsplit(" => ").next().unwrap_or(path).to_string()
        };
        out.insert(path, (a.parse().ok(), r.parse().ok()));
    }
    out
}

/// Line count of a small text file, for untracked files. Large or binary
/// files return `None` so the widget shows no count rather than a bogus one.
fn count_text_lines(path: &std::path::Path) -> Option<usize> {
    const MAX_BYTES: u64 = 2 * 1024 * 1024;
    let meta = std::fs::metadata(path).ok()?;
    if !meta.is_file() || meta.len() > MAX_BYTES {
        return None;
    }
    let bytes = std::fs::read(path).ok()?;
    if bytes.contains(&0) {
        return None;
    }
    let lines = bytes.iter().filter(|&&b| b == b'\n').count();
    Some(if bytes.last().is_some_and(|&b| b != b'\n') {
        lines + 1
    } else {
        lines
    })
}

/// Collapse a porcelain `XY` pair into the single letter the Changes widget
/// shows. Conflicts win, then the worktree side (what the user is editing),
/// then the index side.
pub(crate) fn porcelain_status_letter(index: u8, worktree: u8) -> char {
    if index == b'?' {
        return '?';
    }
    if index == b'U' || worktree == b'U' || (index == b'A' && worktree == b'A') {
        return 'U';
    }
    let pick = if worktree != b' ' { worktree } else { index };
    match pick {
        b'M' | b'T' => 'M',
        b'A' => 'A',
        b'D' => 'D',
        b'R' | b'C' => 'R',
        _ => 'M',
    }
}
