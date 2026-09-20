//! Bounded per-directory cache of working-tree git state.

use super::GitState;
use std::collections::VecDeque;
use std::path::PathBuf;

/// Git state is cached per working directory. A long-lived server or a swarm
/// can touch many directories over its lifetime, so this is bounded rather
/// than growing for the life of the process.
pub(crate) const WORKING_GIT_STATE_CACHE_MAX_ENTRIES: usize = 256;

/// Small LRU over working directories. Capacity is low enough that a linear
/// scan is cheaper than maintaining a second index.
#[derive(Default)]
pub(crate) struct WorkingGitStateCache {
    entries: VecDeque<(PathBuf, Option<GitState>)>,
}

impl WorkingGitStateCache {
    /// Returns the cached value and marks it most recently used.
    pub(crate) fn get(&mut self, key: &PathBuf) -> Option<Option<GitState>> {
        let position = self.entries.iter().position(|(path, _)| path == key)?;
        let entry = self.entries.remove(position)?;
        let value = entry.1.clone();
        self.entries.push_front(entry);
        Some(value)
    }

    pub(crate) fn insert(&mut self, key: PathBuf, value: Option<GitState>) {
        if let Some(position) = self.entries.iter().position(|(path, _)| path == &key) {
            self.entries.remove(position);
        }
        self.entries.push_front((key, value));
        while self.entries.len() > WORKING_GIT_STATE_CACHE_MAX_ENTRIES {
            self.entries.pop_back();
        }
    }

    #[cfg(test)]
    pub(crate) fn len(&self) -> usize {
        self.entries.len()
    }
}
