use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, OnceLock};

static AUTH_REFRESH_GENERATIONS: OnceLock<Mutex<HashMap<String, u64>>> = OnceLock::new();
static NEXT_AUTH_REFRESH_GENERATION: AtomicU64 = AtomicU64::new(1);

pub(super) fn begin_auth_refresh(session_id: &str) -> u64 {
    let generation = NEXT_AUTH_REFRESH_GENERATION.fetch_add(1, Ordering::Relaxed);
    let mut generations = AUTH_REFRESH_GENERATIONS
        .get_or_init(|| Mutex::new(HashMap::new()))
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    generations.insert(session_id.to_string(), generation);
    generation
}

pub(super) fn auth_refresh_is_current(session_id: &str, generation: u64) -> bool {
    let generations = AUTH_REFRESH_GENERATIONS
        .get_or_init(|| Mutex::new(HashMap::new()))
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    generations.get(session_id).copied() == Some(generation)
}

pub(super) fn finish_auth_refresh(session_id: &str, generation: u64) {
    let mut generations = AUTH_REFRESH_GENERATIONS
        .get_or_init(|| Mutex::new(HashMap::new()))
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    if generations.get(session_id).copied() == Some(generation) {
        generations.remove(session_id);
    }
}
