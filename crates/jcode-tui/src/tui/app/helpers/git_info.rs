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
    super::gather_git_info_in(None)
}
