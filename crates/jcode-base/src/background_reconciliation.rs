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
        let guard = self.status_updates.clone().lock_owned().await;
        drop(publish_status_file(&self.status_updates, guard, path, status).await);
    }

    async fn write_status_file_locked(
        &self,
        guard: OwnedMutexGuard<()>,
        path: &std::path::Path,
        status: &TaskStatusFile,
    ) -> OwnedMutexGuard<()> {
        publish_status_file(&self.status_updates, guard, path, status).await
    }

    async fn finalize_detached_status_if_needed(
        &self,
        mut status: TaskStatusFile,
        status_path: &std::path::Path,
    ) -> TaskStatusFile {
        if status.status != BackgroundTaskStatus::Running || !status.detached {
            return status;
        }

        let status_guard = self.status_updates.clone().lock_owned().await;
        let Some(refreshed) = self.read_status_file(status_path).await else {
            return status;
        };
        status = refreshed;
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

        drop(
            self.write_status_file_locked(status_guard, status_path, &status)
                .await,
        );

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

        let status_guard = self.status_updates.clone().lock_owned().await;
        let Some(refreshed) = self.read_status_file(status_path).await else {
            return status;
        };
        status = refreshed;
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
        drop(
            self.write_status_file_locked(status_guard, status_path, &status)
                .await,
        );

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

    /// Cancel a running task
    pub async fn cancel(&self, task_id: &str) -> Result<bool> {
        self.cancel_with_grace(task_id, std::time::Duration::from_millis(400))
            .await
    }

    /// Cancel a running task, allowing detached processes a configurable grace period
    /// between TERM and KILL on Unix.
    pub async fn cancel_with_grace(
        &self,
        task_id: &str,
        _graceful_timeout: std::time::Duration,
    ) -> Result<bool> {
        let status_path = self.status_path_for(task_id);
        let mut status_guard = self.status_updates.clone().lock_owned().await;
        let status = self.read_status_file(&status_path).await;

        if status
            .as_ref()
            .is_some_and(|status| status.status != BackgroundTaskStatus::Running)
        {
            let task = { self.tasks.write().await.remove(task_id) };
            if let Some(task) = task {
                task.handle.abort();
                if let Err(error) = task.handle.await
                    && !error.is_cancelled()
                {
                    crate::logging::warn(&format!("background task join failed: {error}"));
                }
            }
            return Ok(false);
        }

        let task = { self.tasks.write().await.remove(task_id) };
        if let Some(task) = task {
            let RunningTask {
                task_id,
                tool_name,
                display_name,
                session_id,
                started_at,
                started_at_rfc3339,
                delivery_flags,
                handle,
                ..
            } = task;
            handle.abort();

            let (notify_flag, wake_flag) = *delivery_flags.borrow();
            let mut status = status.unwrap_or_else(|| TaskStatusFile {
                task_id: task_id.clone(),
                tool_name: tool_name.clone(),
                display_name: display_name.clone(),
                session_id: session_id.clone(),
                status: BackgroundTaskStatus::Running,
                exit_code: None,
                error: None,
                started_at: started_at_rfc3339.clone(),
                completed_at: None,
                duration_secs: None,
                pid: None,
                owner_pid: Some(std::process::id()),
                owner_instance: Some(model::process_instance_token().to_string()),
                detached: false,
                notify: notify_flag,
                wake: wake_flag,
                progress: None,
                event_history: Vec::new(),
                stall_wake_seconds: None,
            });
            status.task_id = task_id;
            status.tool_name = tool_name;
            status.display_name = status.display_name.or(display_name);
            status.session_id = session_id;
            status.status = BackgroundTaskStatus::Failed;
            status.exit_code = None;
            status.error = Some("Cancelled by user".to_string());
            status.started_at = started_at_rfc3339;
            status.completed_at = Some(Utc::now().to_rfc3339());
            status.duration_secs = Some(started_at.elapsed().as_secs_f64());
            status.pid = None;
            status.owner_pid = Some(std::process::id());
            status.owner_instance = Some(model::process_instance_token().to_string());
            status.detached = false;
            status.notify = notify_flag;
            status.wake = wake_flag;
            status.stall_wake_seconds = None;
            push_task_event(
                &mut status,
                terminal_event_record(
                    BackgroundTaskStatus::Failed,
                    None,
                    Some("Cancelled by user"),
                ),
            );
            status_guard = self
                .write_status_file_locked(status_guard, &status_path, &status)
                .await;
            if let Err(error) = handle.await
                && !error.is_cancelled()
            {
                crate::logging::warn(&format!("background task join failed: {error}"));
            }
            drop(status_guard);
            return Ok(true);
        }

        let Some(mut status) = status else {
            return Ok(false);
        };
        if !status.detached {
            return Ok(false);
        }
        drop(status_guard);
        status = self
            .finalize_detached_status_if_needed(status, &status_path)
            .await;
        if status.status != BackgroundTaskStatus::Running {
            return Ok(false);
        }

        status_guard = self.status_updates.clone().lock_owned().await;
        let Some(refreshed) = self.read_status_file(&status_path).await else {
            return Ok(false);
        };
        status = refreshed;
        if status.status != BackgroundTaskStatus::Running || !status.detached {
            return Ok(false);
        }
        let Some(pid) = status.pid else {
            return Ok(false);
        };

        #[cfg(unix)]
        {
            if let Err(error) = crate::platform::signal_detached_process_group(pid, libc::SIGTERM)
                && error.raw_os_error() != Some(libc::ESRCH)
            {
                crate::logging::warn(&format!("background task {pid} TERM failed: {error}"));
            }
            tokio::time::sleep(_graceful_timeout).await;
            if crate::platform::is_process_running(pid)
                && let Err(error) =
                    crate::platform::signal_detached_process_group(pid, libc::SIGKILL)
                && error.raw_os_error() != Some(libc::ESRCH)
            {
                crate::logging::warn(&format!("background task {pid} KILL failed: {error}"));
            }
        }
        #[cfg(windows)]
        {
            if let Err(error) = crate::platform::signal_detached_process_group(pid, 0) {
                crate::logging::warn(&format!(
                    "background task {pid} termination failed: {error}"
                ));
            }
        }

        let completed_at = Utc::now();
        status.status = BackgroundTaskStatus::Failed;
        status.exit_code = None;
        status.error = Some("Cancelled by user".to_string());
        status.completed_at = Some(completed_at.to_rfc3339());
        status.duration_secs = Self::status_duration_secs(&status.started_at, completed_at);
        push_task_event(
            &mut status,
            terminal_event_record(
                BackgroundTaskStatus::Failed,
                None,
                Some("Cancelled by user"),
            ),
        );
        drop(
            self.write_status_file_locked(status_guard, &status_path, &status)
                .await,
        );
        Ok(true)
    }
}
