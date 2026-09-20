impl BackgroundTaskManager {
    fn status_duration_secs(started_at: &str, completed_at: DateTime<Utc>) -> Option<f64> {
        DateTime::parse_from_rfc3339(started_at)
            .ok()
            .and_then(|started| (completed_at - started.with_timezone(&Utc)).to_std().ok())
            .map(|duration| duration.as_secs_f64())
    }

    fn parse_exit_code_from_output(output: &str) -> Option<i32> {
        output.lines().rev().find_map(|line| {
            let trimmed = line.trim();
            let suffix = trimmed.strip_prefix(EXIT_MARKER_PREFIX)?;
            let suffix = suffix.strip_suffix(" ---")?;
            suffix.trim().parse::<i32>().ok()
        })
    }

    async fn read_status_file(&self, path: &std::path::Path) -> Option<TaskStatusFile> {
        let content = fs::read_to_string(path).await.ok()?;
        serde_json::from_str(&content).ok()
    }

    async fn write_status_file(&self, path: &std::path::Path, status: &TaskStatusFile) {
        if let Ok(json) = serde_json::to_string_pretty(status) {
            let _ = fs::write(path, json).await;
        }
    }

    async fn finalize_detached_status_if_needed(
        &self,
        mut status: TaskStatusFile,
        status_path: &std::path::Path,
    ) -> TaskStatusFile {
        if status.status != BackgroundTaskStatus::Running || !status.detached {
            return status;
        }

        let Some(pid) = status.pid else {
            return status;
        };

        let reaped_exit = crate::platform::try_reap_child_process(pid).ok().flatten();

        if reaped_exit.is_none() && crate::platform::is_process_running(pid) {
            return status;
        }

        let output_path = self.output_path_for(&status.task_id);
        let output = fs::read_to_string(&output_path).await.unwrap_or_default();
        let exit_code = reaped_exit.or_else(|| Self::parse_exit_code_from_output(&output));
        let completed_at = Utc::now();
        let duration_secs = Self::status_duration_secs(&status.started_at, completed_at);
        let final_status = if matches!(exit_code, Some(0)) {
            BackgroundTaskStatus::Completed
        } else {
            BackgroundTaskStatus::Failed
        };
        let final_error = if matches!(final_status, BackgroundTaskStatus::Failed) {
            Some(match exit_code {
                Some(code) => format!("Command exited with code {}", code),
                None => "Detached command exited without a readable exit code".to_string(),
            })
        } else {
            None
        };

        status.status = final_status.clone();
        status.exit_code = exit_code;
        status.error = final_error.clone();
        status.completed_at = Some(completed_at.to_rfc3339());
        status.duration_secs = duration_secs;
        status.pid = Some(pid);
        push_task_event(
            &mut status,
            terminal_event_record(final_status.clone(), exit_code, final_error.as_deref()),
        );

        self.write_status_file(status_path, &status).await;

        let output_preview = if output.len() > 500 {
            format!("{}...", crate::util::truncate_str(&output, 500))
        } else {
            output
        };
        Bus::global().publish(BusEvent::BackgroundTaskCompleted(BackgroundTaskCompleted {
            task_id: status.task_id.clone(),
            tool_name: status.tool_name.clone(),
            display_name: status.display_name.clone(),
            session_id: status.session_id.clone(),
            status: final_status,
            exit_code,
            output_preview,
            output_file: output_path,
            duration_secs: duration_secs.unwrap_or_default(),
            notify: status.notify,
            wake: status.wake,
        }));

        status
    }

    /// True when a non-detached `Running` status file provably belongs to a
    /// process image that no longer exists, so no future can ever finalize it.
    ///
    /// Rules, deliberately conservative because the task dir is shared by
    /// every jcode process on the machine:
    /// - Terminal, detached, or pid-bearing files are never orphans here
    ///   (detached reconciliation is `finalize_detached_status_if_needed`).
    /// - Files owned by this exact process image are never orphans: the
    ///   initial status file is written before the task lands in the live
    ///   map, so "Running + not in map + my instance" can simply mean the
    ///   task is still bootstrapping.
    /// - Files owned by this PID but a different instance token are orphans:
    ///   an exec-based reload replaced the process image, so the owning
    ///   future is gone even though the PID matches.
    /// - Files owned by another PID are orphans only once that process is
    ///   dead.
    /// - Files without owner metadata (written by older builds) are left
    ///   alone; only the explicit startup sweep in
    ///   [`Self::reconcile_orphaned_tasks`] handles those.
    fn status_is_reconcilable_orphan(status: &TaskStatusFile) -> bool {
        if status.status != BackgroundTaskStatus::Running || status.detached || status.pid.is_some()
        {
            return false;
        }
        let Some(owner_pid) = status.owner_pid else {
            return false;
        };
        if status.owner_instance.as_deref() == Some(model::process_instance_token()) {
            return false;
        }
        if owner_pid == std::process::id() {
            return true;
        }
        !crate::platform::is_process_running(owner_pid)
    }

    /// Finalize an orphaned non-detached `Running` status file as `Failed`.
    ///
    /// The owning process's task future died with the process (crash or
    /// exec-based server reload), so without this the file reads `Running`
    /// forever: `bg list`/`bg status` show a phantom task and `bg wait`
    /// blocks until its timeout.
    async fn finalize_orphaned_status_if_needed(
        &self,
        mut status: TaskStatusFile,
        status_path: &std::path::Path,
    ) -> TaskStatusFile {
        if !Self::status_is_reconcilable_orphan(&status) {
            return status;
        }
        // Belt and braces: never rewrite a task this process is executing.
        if self.is_live_task(&status.task_id) {
            return status;
        }

        let completed_at = Utc::now();
        let duration_secs = Self::status_duration_secs(&status.started_at, completed_at);
        let error =
            "Task orphaned: the owning server process exited (reloaded or crashed) before the task finished"
                .to_string();
        status.status = BackgroundTaskStatus::Failed;
        status.exit_code = None;
        status.error = Some(error.clone());
        status.completed_at = Some(completed_at.to_rfc3339());
        status.duration_secs = duration_secs;
        push_task_event(
            &mut status,
            terminal_event_record(BackgroundTaskStatus::Failed, None, Some(&error)),
        );
        self.write_status_file(status_path, &status).await;

        let output_path = self.output_path_for(&status.task_id);
        let output = fs::read_to_string(&output_path).await.unwrap_or_default();
        let output_preview = if output.len() > 500 {
            format!("{}...", crate::util::truncate_str(&output, 500))
        } else {
            output
        };
        Bus::global().publish(BusEvent::BackgroundTaskCompleted(BackgroundTaskCompleted {
            task_id: status.task_id.clone(),
            tool_name: status.tool_name.clone(),
            display_name: status.display_name.clone(),
            session_id: status.session_id.clone(),
            status: BackgroundTaskStatus::Failed,
            exit_code: None,
            output_preview,
            output_file: output_path,
            duration_secs: duration_secs.unwrap_or_default(),
            notify: status.notify,
            wake: status.wake,
        }));

        status
    }

    /// Startup/reload sweep: mark orphaned non-detached `Running` status
    /// files as `Failed` with a "server reloaded" note.
    ///
    /// Only owner-tagged files are considered, using the liveness rules of
    /// [`Self::status_is_reconcilable_orphan`]. Files without owner metadata
    /// (written by older builds, or by processes that legitimately still run
    /// them) are left untouched: the task dir is shared machine-wide, so
    /// without owner metadata there is no safe way to distinguish a phantom
    /// from another live process's task. Returns how many files were
    /// reconciled.
    pub async fn reconcile_orphaned_tasks(&self) -> usize {
        let mut reconciled = 0;
        let Ok(mut entries) = fs::read_dir(&self.output_dir).await else {
            return reconciled;
        };
        while let Ok(Some(entry)) = entries.next_entry().await {
            let path = entry.path();
            if path.extension().and_then(|ext| ext.to_str()) != Some("json") {
                continue;
            }
            let Some(status) = self.read_status_file(&path).await else {
                continue;
            };
            if !Self::status_is_reconcilable_orphan(&status) {
                continue;
            }
            if self.tasks.read().await.contains_key(&status.task_id) {
                continue;
            }
            self.finalize_orphaned_status_if_needed(status, &path).await;
            reconciled += 1;
        }
        reconciled
    }
}
