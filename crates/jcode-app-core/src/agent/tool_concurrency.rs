use super::*;
use std::path::Path;
use tokio::sync::Semaphore;
use tokio::task::JoinHandle;

const MAX_PARALLEL_TOOLS: usize = 4;
type SdkToolResults = HashMap<String, (String, bool)>;

struct ParallelToolTask {
    call: ToolCall,
    started_at: Instant,
    handle: Option<JoinHandle<ToolTaskOutcome>>,
}

struct ToolTaskOutcome {
    result: Result<crate::tool::ToolOutput>,
    elapsed: Duration,
}

struct AbortOnDropToolHandle {
    handle: Option<JoinHandle<ToolTaskOutcome>>,
}

impl AbortOnDropToolHandle {
    async fn wait(mut self) -> Result<crate::tool::ToolOutput> {
        let Some(handle) = self.handle.as_mut() else {
            return Err(anyhow::anyhow!("Tool task handle was already consumed"));
        };
        let joined = handle.await;
        self.handle.take();
        match joined {
            Ok(outcome) => outcome.result,
            Err(error) => Err(anyhow::anyhow!("Tool task panicked: {error}")),
        }
    }
}

impl Drop for AbortOnDropToolHandle {
    fn drop(&mut self) {
        if let Some(handle) = self.handle.take() {
            handle.abort();
        }
    }
}

async fn finish_background_tool(
    owner: AbortOnDropToolHandle,
    output_path: PathBuf,
) -> Result<crate::background::TaskResult> {
    let (output, error, exit_code) = match owner.wait().await {
        Ok(output) => (output.output, None, Some(0)),
        Err(error) => {
            let error = error.to_string();
            (error.clone(), Some(error), None)
        }
    };
    tokio::fs::write(output_path, output).await?;
    Ok(crate::background::TaskResult {
        exit_code,
        error,
        status: None,
    })
}

impl ParallelToolTask {
    fn is_finished(&self) -> bool {
        self.handle.as_ref().is_none_or(JoinHandle::is_finished)
    }

    async fn join(&mut self) -> ToolTaskOutcome {
        let Some(handle) = self.handle.as_mut() else {
            return ToolTaskOutcome {
                result: Err(anyhow::anyhow!("Tool task handle was already consumed")),
                elapsed: self.started_at.elapsed(),
            };
        };
        let joined = handle.await;
        self.handle.take();
        joined.unwrap_or_else(|error| ToolTaskOutcome {
            result: Err(anyhow::anyhow!("Tool task panicked: {error}")),
            elapsed: self.started_at.elapsed(),
        })
    }

    fn abort(&mut self) {
        if let Some(handle) = self.handle.take() {
            handle.abort();
        }
    }

    fn take_background_owner(&mut self) -> Option<AbortOnDropToolHandle> {
        Some(AbortOnDropToolHandle {
            handle: Some(self.handle.take()?),
        })
    }
}

impl Drop for ParallelToolTask {
    fn drop(&mut self) {
        self.abort();
    }
}

struct ParallelToolTasks {
    tasks: Vec<ParallelToolTask>,
}

impl ParallelToolTasks {
    fn spawn(agent: &Agent, calls: &[ToolCall], message_id: &str) -> Self {
        let semaphore = Arc::new(Semaphore::new(MAX_PARALLEL_TOOLS));
        let tasks = calls
            .iter()
            .cloned()
            .map(|call| {
                let registry = agent.registry.clone();
                let semaphore = semaphore.clone();
                let tool_name = call.name.clone();
                let tool_input = call.input.clone();
                let ctx = ToolContext {
                    session_id: agent.session.id.clone(),
                    message_id: message_id.to_string(),
                    tool_call_id: call.id.clone(),
                    working_dir: agent.working_dir().map(PathBuf::from),
                    stdin_request_tx: agent.stdin_request_tx.clone(),
                    graceful_shutdown_signal: Some(agent.graceful_shutdown.clone()),
                    execution_mode: ToolExecutionMode::AgentTurn,
                };
                let handle = tokio::spawn(async move {
                    let Ok(_permit) = semaphore.acquire_owned().await else {
                        return ToolTaskOutcome {
                            result: Err(anyhow::anyhow!("Tool concurrency limiter closed")),
                            elapsed: Duration::ZERO,
                        };
                    };
                    let started_at = Instant::now();
                    let result = registry.execute(&tool_name, tool_input, ctx).await;
                    ToolTaskOutcome {
                        result,
                        elapsed: started_at.elapsed(),
                    }
                });
                ParallelToolTask {
                    call,
                    started_at: Instant::now(),
                    handle: Some(handle),
                }
            })
            .collect();
        Self { tasks }
    }
}

fn read_is_parallel_safe(input: &serde_json::Value) -> bool {
    let Some(path) = input.get("file_path").and_then(|value| value.as_str()) else {
        return false;
    };
    let extension = Path::new(path)
        .extension()
        .map(|extension| extension.to_string_lossy().to_lowercase())
        .unwrap_or_default();
    !matches!(
        extension.as_str(),
        "png" | "jpg" | "jpeg" | "gif" | "webp" | "bmp" | "ico" | "pdf"
    )
}

fn tool_is_parallel_safe(call: &ToolCall) -> bool {
    if crate::hooks::hook_configured("pre_tool") || crate::hooks::hook_configured("post_tool") {
        return false;
    }
    match Registry::resolve_tool_name(&call.name) {
        "read" => read_is_parallel_safe(&call.input),
        "ls" | "jcode_docs" => true,
        _ => false,
    }
}

impl Agent {
    fn parallel_run_end(
        &self,
        calls: &[ToolCall],
        start: usize,
        sdk_results: &SdkToolResults,
    ) -> usize {
        let mut end = start;
        while let Some(call) = calls.get(end) {
            if !tool_is_parallel_safe(call)
                || call.validation_error().is_some()
                || sdk_results.contains_key(&call.id)
                || self.validate_tool_allowed(&call.name).is_err()
            {
                break;
            }
            end += 1;
        }
        end
    }

    fn blocking_tool_started(&self, call: &ToolCall, message_id: &str, trace: bool) {
        if trace {
            eprintln!("[trace] tool_exec_start name={} id={}", call.name, call.id);
        }
        Bus::global().publish(BusEvent::ToolUpdated(ToolEvent {
            session_id: self.session.id.clone(),
            message_id: message_id.to_string(),
            tool_call_id: call.id.clone(),
            tool_name: call.name.clone(),
            status: ToolStatus::Running,
            intent: call.intent.clone(),
            title: None,
        }));
        logging::info(&format!("Tool starting: {}", call.name));
        Bus::global().publish(BusEvent::SubagentStatus(SubagentStatus {
            session_id: self.session.id.clone(),
            status: format!("running {}", call.name),
            model: Some(self.provider.model()),
        }));
    }

    fn finish_blocking_tool(
        &mut self,
        call: ToolCall,
        message_id: &str,
        result: Result<crate::tool::ToolOutput>,
        elapsed: Duration,
        print_output: bool,
        trace: bool,
    ) {
        logging::info(&format!(
            "Tool finished: {} in {:.2}s",
            call.name,
            elapsed.as_secs_f64()
        ));
        match result {
            Ok(output) => {
                let output = cap_tool_output_for_history(&call.name, output);
                Bus::global().publish(BusEvent::ToolUpdated(ToolEvent {
                    session_id: self.session.id.clone(),
                    message_id: message_id.to_string(),
                    tool_call_id: call.id.clone(),
                    tool_name: call.name.clone(),
                    status: ToolStatus::Completed,
                    intent: call.intent.clone(),
                    title: output.title.clone(),
                }));
                if trace {
                    eprintln!(
                        "[trace] tool_exec_done name={} id={}\n{}",
                        call.name, call.id, output.output
                    );
                }
                if print_output {
                    let preview = if output.output.len() > 200 {
                        format!("{}...", crate::util::truncate_str(&output.output, 200))
                    } else {
                        output.output.clone()
                    };
                    println!("{}", preview.lines().next().unwrap_or("(done)"));
                }
                let blocks = tool_output_to_content_blocks(call.id, output);
                self.add_message_with_duration(
                    Role::User,
                    blocks,
                    Some(elapsed.as_millis() as u64),
                );
            }
            Err(error) => {
                crate::telemetry::record_tool_failure();
                Bus::global().publish(BusEvent::ToolUpdated(ToolEvent {
                    session_id: self.session.id.clone(),
                    message_id: message_id.to_string(),
                    tool_call_id: call.id.clone(),
                    tool_name: call.name.clone(),
                    status: ToolStatus::Error,
                    intent: call.intent.clone(),
                    title: None,
                }));
                let error_message = format!("Error: {error}");
                if trace {
                    eprintln!(
                        "[trace] tool_exec_error name={} id={} {}",
                        call.name, call.id, error_message
                    );
                }
                if print_output {
                    println!("{}", error_message);
                }
                self.add_message_with_duration(
                    Role::User,
                    vec![ContentBlock::ToolResult {
                        tool_use_id: call.id,
                        content: error_message,
                        is_error: Some(true),
                    }],
                    Some(elapsed.as_millis() as u64),
                );
            }
        }
    }

    fn skip_blocking_tool(&mut self, call: &ToolCall, message_id: &str, content: &str) {
        Bus::global().publish(BusEvent::ToolUpdated(ToolEvent {
            session_id: self.session.id.clone(),
            message_id: message_id.to_string(),
            tool_call_id: call.id.clone(),
            tool_name: call.name.clone(),
            status: ToolStatus::Error,
            intent: call.intent.clone(),
            title: None,
        }));
        self.add_message(
            Role::User,
            vec![ContentBlock::ToolResult {
                tool_use_id: call.id.clone(),
                content: content.to_string(),
                is_error: Some(true),
            }],
        );
    }

    async fn execute_one_blocking_tool(
        &mut self,
        call: ToolCall,
        message_id: &str,
        sdk_results: &mut SdkToolResults,
        print_output: bool,
        trace: bool,
    ) -> Result<()> {
        if let Some(error_message) = call.validation_error() {
            logging::warn(&error_message);
            Bus::global().publish(BusEvent::ToolUpdated(ToolEvent {
                session_id: self.session.id.clone(),
                message_id: message_id.to_string(),
                tool_call_id: call.id.clone(),
                tool_name: call.name.clone(),
                status: ToolStatus::Error,
                intent: call.intent.clone(),
                title: None,
            }));
            if print_output {
                println!("\n  → {}", error_message);
            }
            self.add_message(
                Role::User,
                vec![ContentBlock::ToolResult {
                    tool_use_id: call.id,
                    content: error_message,
                    is_error: Some(true),
                }],
            );
            return Ok(());
        }
        self.validate_tool_allowed(&call.name)?;
        let is_native_tool = JCODE_NATIVE_TOOLS.contains(&call.name.as_str());
        if let Some((sdk_content, sdk_is_error)) = sdk_results.remove(&call.id)
            && !(is_native_tool && sdk_is_error)
        {
            if print_output {
                print!("\n  → ");
                let preview = if sdk_content.len() > 200 {
                    format!("{}...", crate::util::truncate_str(&sdk_content, 200))
                } else {
                    sdk_content.clone()
                };
                println!("{}", preview.lines().next().unwrap_or("(done via SDK)"));
            }
            Bus::global().publish(BusEvent::ToolUpdated(ToolEvent {
                session_id: self.session.id.clone(),
                message_id: message_id.to_string(),
                tool_call_id: call.id.clone(),
                tool_name: call.name.clone(),
                status: if sdk_is_error {
                    ToolStatus::Error
                } else {
                    ToolStatus::Completed
                },
                intent: call.intent.clone(),
                title: None,
            }));
            self.add_message(
                Role::User,
                vec![ContentBlock::ToolResult {
                    tool_use_id: call.id,
                    content: sdk_content,
                    is_error: sdk_is_error.then_some(true),
                }],
            );
            return Ok(());
        }
        if print_output {
            print!("\n  → ");
            io::stdout().flush()?;
        }
        self.blocking_tool_started(&call, message_id, trace);
        let started_at = Instant::now();
        let ctx = ToolContext {
            session_id: self.session.id.clone(),
            message_id: message_id.to_string(),
            tool_call_id: call.id.clone(),
            working_dir: self.working_dir().map(PathBuf::from),
            stdin_request_tx: self.stdin_request_tx.clone(),
            graceful_shutdown_signal: Some(self.graceful_shutdown.clone()),
            execution_mode: ToolExecutionMode::AgentTurn,
        };
        let result = self
            .registry
            .execute(&call.name, call.input.clone(), ctx)
            .await;
        crate::telemetry::record_tool_call();
        self.unlock_tools_if_needed(&call.name);
        self.finish_blocking_tool(
            call,
            message_id,
            result,
            started_at.elapsed(),
            print_output,
            trace,
        );
        Ok(())
    }

    pub(super) async fn execute_blocking_tool_calls(
        &mut self,
        calls: Vec<ToolCall>,
        assistant_message_id: Option<&str>,
        sdk_results: &mut SdkToolResults,
        print_output: bool,
        trace: bool,
    ) -> Result<()> {
        let message_id = assistant_message_id
            .map(ToString::to_string)
            .unwrap_or_else(|| self.session.id.clone());
        let mut index = 0;
        while index < calls.len() {
            let end = self.parallel_run_end(&calls, index, sdk_results);
            if end.saturating_sub(index) < 2 {
                self.execute_one_blocking_tool(
                    calls[index].clone(),
                    &message_id,
                    sdk_results,
                    print_output,
                    trace,
                )
                .await?;
                index += 1;
                continue;
            }
            if print_output {
                print!("\n  → ");
                io::stdout().flush()?;
            }
            for call in &calls[index..end] {
                self.blocking_tool_started(call, &message_id, trace);
            }
            let mut tasks = ParallelToolTasks::spawn(self, &calls[index..end], &message_id);
            let mut interrupted = false;
            for task in &mut tasks.tasks {
                let result = tokio::select! {
                    biased;
                    joined = task.join() => Some(joined),
                    _ = self.graceful_shutdown.notified() => None,
                };
                if let Some(outcome) = result {
                    crate::telemetry::record_tool_call();
                    self.unlock_tools_if_needed(&task.call.name);
                    self.finish_blocking_tool(
                        task.call.clone(),
                        &message_id,
                        outcome.result,
                        outcome.elapsed,
                        print_output,
                        trace,
                    );
                } else {
                    interrupted = true;
                    break;
                }
            }
            if interrupted {
                for task in &mut tasks.tasks {
                    if task.handle.is_none() {
                        continue;
                    }
                    crate::telemetry::record_tool_call();
                    self.unlock_tools_if_needed(&task.call.name);
                    if task.is_finished() {
                        let outcome = task.join().await;
                        self.finish_blocking_tool(
                            task.call.clone(),
                            &message_id,
                            outcome.result,
                            outcome.elapsed,
                            print_output,
                            trace,
                        );
                    } else {
                        task.abort();
                        self.skip_blocking_tool(
                            &task.call,
                            &message_id,
                            "[Skipped - server reloading]",
                        );
                    }
                }
                for call in &calls[end..] {
                    self.skip_blocking_tool(call, &message_id, "[Skipped - server reloading]");
                }
                break;
            }
            index = end;
        }
        self.session.save()?;
        Ok(())
    }

    fn send_streaming_tool_done(
        event_tx: &mpsc::UnboundedSender<ServerEvent>,
        call: &ToolCall,
        output: String,
        error: Option<String>,
    ) {
        let _ = event_tx.send(ServerEvent::ToolDone {
            id: call.id.clone(),
            name: call.name.clone(),
            output,
            error,
        });
    }

    fn finish_streaming_tool(
        &mut self,
        call: &ToolCall,
        result: Result<crate::tool::ToolOutput>,
        elapsed: Duration,
        event_tx: &mpsc::UnboundedSender<ServerEvent>,
        inline_output_tap: bool,
    ) {
        logging::info(&format!(
            "Tool finished: {} in {:.2}s",
            call.name,
            elapsed.as_secs_f64()
        ));
        if inline_output_tap {
            self.inline_tail
                .finish_tool(elapsed.as_secs_f64(), result.is_err());
            self.publish_inline_tail();
        }
        match result {
            Ok(output) => {
                let output = cap_tool_output_for_history(&call.name, output);
                Self::send_streaming_tool_done(event_tx, call, output.output.clone(), None);
                let images =
                    tool_output_side_pane_images(&call.id, &call.name, &call.input, &output);
                if !images.is_empty() {
                    let _ = event_tx.send(ServerEvent::SidePaneImages {
                        session_id: self.session.id.clone(),
                        images,
                    });
                }
                self.add_message_with_duration(
                    Role::User,
                    tool_output_to_content_blocks(call.id.clone(), output),
                    Some(elapsed.as_millis() as u64),
                );
            }
            Err(error) => {
                let error_message = format!("Error: {error}");
                Self::send_streaming_tool_done(
                    event_tx,
                    call,
                    error_message.clone(),
                    Some(error_message.clone()),
                );
                self.add_message_with_duration(
                    Role::User,
                    vec![ContentBlock::ToolResult {
                        tool_use_id: call.id.clone(),
                        content: error_message,
                        is_error: Some(true),
                    }],
                    Some(elapsed.as_millis() as u64),
                );
            }
        }
    }

    fn skip_streaming_tools(
        &mut self,
        calls: &[ToolCall],
        content: &str,
        error: &str,
        event_tx: &mpsc::UnboundedSender<ServerEvent>,
    ) {
        for call in calls {
            Self::send_streaming_tool_done(
                event_tx,
                call,
                content.to_string(),
                Some(error.to_string()),
            );
            self.add_message(
                Role::User,
                vec![ContentBlock::ToolResult {
                    tool_use_id: call.id.clone(),
                    content: content.to_string(),
                    is_error: Some(true),
                }],
            );
        }
    }

    async fn execute_one_streaming_tool(
        &mut self,
        call: &ToolCall,
        message_id: &str,
        sdk_results: &mut SdkToolResults,
        event_tx: &mpsc::UnboundedSender<ServerEvent>,
        inline_output_tap: bool,
        trace: bool,
    ) -> Result<bool> {
        if let Some(error_message) = call.validation_error() {
            logging::warn(&error_message);
            Self::send_streaming_tool_done(
                event_tx,
                call,
                error_message.clone(),
                Some(error_message.clone()),
            );
            self.add_message(
                Role::User,
                vec![ContentBlock::ToolResult {
                    tool_use_id: call.id.clone(),
                    content: error_message,
                    is_error: Some(true),
                }],
            );
            return Ok(false);
        }
        self.validate_tool_allowed(&call.name)?;
        let is_native_tool = JCODE_NATIVE_TOOLS.contains(&call.name.as_str());
        if let Some((sdk_content, sdk_is_error)) = sdk_results.remove(&call.id)
            && !(is_native_tool && sdk_is_error)
        {
            let sdk_content = cap_sdk_tool_content_for_history(&call.name, sdk_content);
            self.add_message(
                Role::User,
                vec![ContentBlock::ToolResult {
                    tool_use_id: call.id.clone(),
                    content: sdk_content,
                    is_error: sdk_is_error.then_some(true),
                }],
            );
            return Ok(false);
        }
        let ctx = ToolContext {
            session_id: self.session.id.clone(),
            message_id: message_id.to_string(),
            tool_call_id: call.id.clone(),
            working_dir: self.working_dir().map(PathBuf::from),
            stdin_request_tx: self.stdin_request_tx.clone(),
            graceful_shutdown_signal: Some(self.graceful_shutdown.clone()),
            execution_mode: ToolExecutionMode::AgentTurn,
        };
        if trace {
            eprintln!("[trace] tool_exec_start name={} id={}", call.name, call.id);
        }
        logging::info(&format!("Tool starting: {}", call.name));
        crate::session_metrics::record_activity(&self.session.id);
        if inline_output_tap {
            self.inline_tail.start_tool(&call.name, &call.input);
            self.publish_inline_tail();
        }
        let started_at = Instant::now();
        let registry = self.registry.clone();
        let tool_name = call.name.clone();
        let tool_input = call.input.clone();
        let mut task = ParallelToolTask {
            call: call.clone(),
            started_at,
            handle: Some(tokio::spawn(async move {
                let started_at = Instant::now();
                let result = registry.execute(&tool_name, tool_input, ctx).await;
                ToolTaskOutcome {
                    result,
                    elapsed: started_at.elapsed(),
                }
            })),
        };
        self.background_tool_signal.reset();
        let mut result = tokio::select! {
            biased;
            joined = task.join() => Some(joined),
            _ = self.graceful_shutdown.notified() => None,
            _ = self.background_tool_signal.notified() => None,
        };
        if result.is_none() && self.is_graceful_shutdown() && call.name == "bash" {
            result = tokio::time::timeout(Duration::from_millis(750), task.join())
                .await
                .ok();
        }
        self.unlock_tools_if_needed(&call.name);
        let elapsed = started_at.elapsed();
        crate::session_metrics::record_activity(&self.session.id);
        if let Some(outcome) = result {
            self.finish_streaming_tool(
                call,
                outcome.result,
                outcome.elapsed,
                event_tx,
                inline_output_tap,
            );
            return Ok(false);
        }
        if self.is_graceful_shutdown() {
            task.abort();
            let (message, is_error) = super::turn_streaming_mpsc::reload_interrupted_tool_result(
                call,
                elapsed.as_secs_f64(),
            );
            Self::send_streaming_tool_done(
                event_tx,
                call,
                message.clone(),
                is_error.then(|| "interrupted by reload".to_string()),
            );
            self.add_message_with_duration(
                Role::User,
                vec![ContentBlock::ToolResult {
                    tool_use_id: call.id.clone(),
                    content: message,
                    is_error: Some(is_error),
                }],
                Some(elapsed.as_millis() as u64),
            );
            return Ok(true);
        }
        let Some(owner) = task.take_background_owner() else {
            self.finish_streaming_tool(
                call,
                Err(anyhow::anyhow!("Tool task handle was already consumed")),
                elapsed,
                event_tx,
                inline_output_tap,
            );
            return Ok(false);
        };
        let info = crate::background::global()
            .spawn(&call.name, &self.session.id, move |output_path| {
                finish_background_tool(owner, output_path)
            })
            .await;
        let message = format!(
            "Tool '{}' was moved to background by the user (task_id: {}). Use the `bg` tool with action 'wait' to wait for completion/checkpoints, or action 'status'/'output' to inspect it.",
            call.name, info.task_id
        );
        Self::send_streaming_tool_done(event_tx, call, message.clone(), None);
        self.add_message_with_duration(
            Role::User,
            vec![ContentBlock::ToolResult {
                tool_use_id: call.id.clone(),
                content: message,
                is_error: None,
            }],
            Some(elapsed.as_millis() as u64),
        );
        self.background_tool_signal.reset();
        Ok(false)
    }

    async fn finish_parallel_streaming_group(
        &mut self,
        tasks: &mut ParallelToolTasks,
        event_tx: &mpsc::UnboundedSender<ServerEvent>,
        inline_output_tap: bool,
    ) -> Result<Option<bool>> {
        for index in 0..tasks.tasks.len() {
            if inline_output_tap {
                let call = &tasks.tasks[index].call;
                self.inline_tail.start_tool(&call.name, &call.input);
                self.publish_inline_tail();
            }
            let result = tokio::select! {
                biased;
                joined = tasks.tasks[index].join() => Some(joined),
                _ = self.graceful_shutdown.notified() => None,
                _ = self.background_tool_signal.notified() => None,
            };
            let call = tasks.tasks[index].call.clone();
            self.unlock_tools_if_needed(&call.name);
            crate::session_metrics::record_activity(&self.session.id);
            if let Some(outcome) = result {
                self.finish_streaming_tool(
                    &call,
                    outcome.result,
                    outcome.elapsed,
                    event_tx,
                    inline_output_tap,
                );
                continue;
            }
            if self.is_graceful_shutdown() {
                for (offset, task) in tasks.tasks[index..].iter_mut().enumerate() {
                    let call = task.call.clone();
                    let finish_inline = inline_output_tap && offset == 0;
                    if task.is_finished() {
                        let outcome = task.join().await;
                        self.unlock_tools_if_needed(&call.name);
                        self.finish_streaming_tool(
                            &call,
                            outcome.result,
                            outcome.elapsed,
                            event_tx,
                            finish_inline,
                        );
                    } else {
                        task.abort();
                        self.unlock_tools_if_needed(&call.name);
                        let elapsed = task.started_at.elapsed();
                        let (message, is_error) =
                            super::turn_streaming_mpsc::reload_interrupted_tool_result(
                                &call,
                                elapsed.as_secs_f64(),
                            );
                        if finish_inline {
                            self.inline_tail
                                .finish_tool(elapsed.as_secs_f64(), is_error);
                            self.publish_inline_tail();
                        }
                        Self::send_streaming_tool_done(
                            event_tx,
                            &call,
                            message.clone(),
                            is_error.then(|| "interrupted by reload".to_string()),
                        );
                        self.add_message_with_duration(
                            Role::User,
                            vec![ContentBlock::ToolResult {
                                tool_use_id: call.id,
                                content: message,
                                is_error: Some(is_error),
                            }],
                            Some(elapsed.as_millis() as u64),
                        );
                    }
                }
                return Ok(Some(true));
            }
            for (offset, task) in tasks.tasks[index..].iter_mut().enumerate() {
                let call = task.call.clone();
                let finish_inline = inline_output_tap && offset == 0;
                if task.is_finished() {
                    let outcome = task.join().await;
                    self.unlock_tools_if_needed(&call.name);
                    self.finish_streaming_tool(
                        &call,
                        outcome.result,
                        outcome.elapsed,
                        event_tx,
                        finish_inline,
                    );
                    continue;
                }
                self.unlock_tools_if_needed(&call.name);
                let elapsed = task.started_at.elapsed();
                let Some(owner) = task.take_background_owner() else {
                    self.finish_streaming_tool(
                        &call,
                        Err(anyhow::anyhow!("Tool task handle was already consumed")),
                        elapsed,
                        event_tx,
                        finish_inline,
                    );
                    continue;
                };
                let info = crate::background::global()
                    .spawn(&call.name, &self.session.id, move |output_path| {
                        finish_background_tool(owner, output_path)
                    })
                    .await;
                let message = format!(
                    "Tool '{}' was moved to background by the user (task_id: {}). Use the `bg` tool with action 'wait' to wait for completion/checkpoints, or action 'status'/'output' to inspect it.",
                    call.name, info.task_id
                );
                Self::send_streaming_tool_done(event_tx, &call, message.clone(), None);
                self.add_message_with_duration(
                    Role::User,
                    vec![ContentBlock::ToolResult {
                        tool_use_id: call.id,
                        content: message,
                        is_error: None,
                    }],
                    Some(elapsed.as_millis() as u64),
                );
                if finish_inline {
                    self.inline_tail.finish_tool(elapsed.as_secs_f64(), false);
                    self.publish_inline_tail();
                }
            }
            self.background_tool_signal.reset();
            return Ok(Some(false));
        }
        Ok(None)
    }

    pub(super) async fn execute_streaming_tool_calls(
        &mut self,
        calls: Vec<ToolCall>,
        assistant_message_id: Option<&str>,
        sdk_results: &mut SdkToolResults,
        event_tx: &mpsc::UnboundedSender<ServerEvent>,
        inline_output_tap: bool,
        trace: bool,
    ) -> Result<bool> {
        let message_id = assistant_message_id
            .map(ToString::to_string)
            .unwrap_or_else(|| self.session.id.clone());
        let mut index = 0;
        while index < calls.len() {
            if index > 0 && self.has_urgent_interrupt() {
                crate::telemetry::record_user_cancelled();
                let remaining = &calls[index..];
                self.skip_streaming_tools(
                    remaining,
                    "[Skipped: user interrupted]",
                    "user interrupted",
                    event_tx,
                );
                let injected = self.inject_soft_interrupts();
                if !injected.is_empty() {
                    for event in
                        Self::build_soft_interrupt_events(injected, "C", Some(remaining.len()))
                    {
                        let _ = event_tx.send(event);
                    }
                    self.add_message(
                        Role::User,
                        vec![ContentBlock::Text {
                            text: format!(
                                "[User interrupted: {} remaining tool(s) skipped]",
                                remaining.len()
                            ),
                            cache_control: None,
                        }],
                    );
                }
                break;
            }
            let end = self.parallel_run_end(&calls, index, sdk_results);
            if end.saturating_sub(index) < 2 {
                let reload = self
                    .execute_one_streaming_tool(
                        &calls[index],
                        &message_id,
                        sdk_results,
                        event_tx,
                        inline_output_tap,
                        trace,
                    )
                    .await?;
                if reload {
                    self.skip_streaming_tools(
                        &calls[(index + 1)..],
                        "[Skipped - server reloading]",
                        "server reloading",
                        event_tx,
                    );
                    self.session.save()?;
                    return Ok(true);
                }
                index += 1;
                continue;
            }
            for call in &calls[index..end] {
                if trace {
                    eprintln!("[trace] tool_exec_start name={} id={}", call.name, call.id);
                }
                logging::info(&format!("Tool starting: {}", call.name));
            }
            crate::session_metrics::record_activity(&self.session.id);
            self.background_tool_signal.reset();
            let mut tasks = ParallelToolTasks::spawn(self, &calls[index..end], &message_id);
            if self
                .finish_parallel_streaming_group(&mut tasks, event_tx, inline_output_tap)
                .await?
                == Some(true)
            {
                self.skip_streaming_tools(
                    &calls[end..],
                    "[Skipped - server reloading]",
                    "server reloading",
                    event_tx,
                );
                self.session.save()?;
                return Ok(true);
            }
            index = end;
        }
        self.session.save()?;
        Ok(false)
    }
}

#[cfg(test)]
#[path = "tool_concurrency_tests.rs"]
mod tests;
