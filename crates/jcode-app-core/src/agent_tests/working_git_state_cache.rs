use super::*;
use crate::agent::working_git_state_cache::WORKING_GIT_STATE_CACHE_MAX_ENTRIES;
use std::path::PathBuf;

#[test]
fn working_git_state_cache_evicts_beyond_capacity() {
    let mut cache = WorkingGitStateCache::default();

    // A long-lived server or swarm can touch far more directories than the
    // cache should ever retain.
    for index in 0..(WORKING_GIT_STATE_CACHE_MAX_ENTRIES * 3) {
        cache.insert(PathBuf::from(format!("/tmp/repo-{index}")), None);
    }

    assert_eq!(cache.len(), WORKING_GIT_STATE_CACHE_MAX_ENTRIES);
}

#[test]
fn working_git_state_cache_keeps_recently_used_entries() {
    let mut cache = WorkingGitStateCache::default();
    let hot = PathBuf::from("/tmp/hot-repo");
    cache.insert(hot.clone(), None);

    // Touch the hot entry as it is about to age out, then overflow the cache.
    for index in 0..(WORKING_GIT_STATE_CACHE_MAX_ENTRIES - 1) {
        cache.insert(PathBuf::from(format!("/tmp/repo-{index}")), None);
    }
    assert!(
        cache.get(&hot).is_some(),
        "hot entry should still be cached"
    );
    for index in 0..WORKING_GIT_STATE_CACHE_MAX_ENTRIES {
        cache.insert(PathBuf::from(format!("/tmp/late-{index}")), None);
    }

    assert!(
        cache.get(&hot).is_none(),
        "an entry not touched during a full turnover should be evicted"
    );
    assert_eq!(cache.len(), WORKING_GIT_STATE_CACHE_MAX_ENTRIES);
}

#[test]
fn working_git_state_cache_reinsert_does_not_duplicate() {
    let mut cache = WorkingGitStateCache::default();
    let path = PathBuf::from("/tmp/repo");

    cache.insert(path.clone(), None);
    cache.insert(path.clone(), None);

    assert_eq!(cache.len(), 1);
}
