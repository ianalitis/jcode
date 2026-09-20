/// Progress state for a foreground command that may be promoted to a
/// background task if it exceeds the foreground timeout.
///
/// Before promotion there is no task to attach progress to, so only the most
/// recent update is kept. When the command is promoted, `attach_task` flushes
/// that pending update so the task row starts at the real percentage instead
/// of 0%, and later updates stream directly to the background manager.
#[derive(Default)]
struct PromotedCommandProgress {
    task_id: std::sync::OnceLock<String>,
    pending: std::sync::Mutex<Option<ProgressLineUpdate>>,
}

impl PromotedCommandProgress {
    /// Lock the pending progress slot, recovering from a poisoned mutex.
    ///
    /// Progress bookkeeping must never abort a command, so a panicking writer
    /// cannot justify propagating the poison to the reader.
    fn lock_pending(&self) -> std::sync::MutexGuard<'_, Option<ProgressLineUpdate>> {
        match self.pending.lock() {
            Ok(guard) => guard,
            Err(poisoned) => {
                crate::logging::warn("bash progress mutex was poisoned; recovering state");
                poisoned.into_inner()
            }
        }
    }

    async fn record(&self, update: ProgressLineUpdate) {
        let direct = {
            let mut pending = self.lock_pending();
            if self.task_id.get().is_none() {
                *pending = Some(update);
                None
            } else {
                Some(update)
            }
        };
        if let Some(update) = direct
            && let Some(task_id) = self.task_id.get()
        {
            apply_progress_update(task_id, update).await;
        }
    }

    async fn attach_task(&self, task_id: &str) {
        let _ = self.task_id.set(task_id.to_string());
        let pending = self.lock_pending().take();
        if let Some(update) = pending {
            apply_progress_update(task_id, update).await;
        }
    }
}
