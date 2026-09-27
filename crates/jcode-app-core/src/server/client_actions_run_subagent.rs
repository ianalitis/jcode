pub(super) fn handle_run_subagent(
    id: u64,
    prompt: String,
    subagent_type: String,
    model: Option<String>,
    session_id: Option<String>,
    agent: &Arc<Mutex<Agent>>,
    client_event_tx: &mpsc::UnboundedSender<ServerEvent>,
) {
    let agent = Arc::clone(agent);
    let tx = client_event_tx.clone();

    tokio::spawn(async move {
        let description = derive_subagent_description(&prompt);
        let tool_call_id = crate::id::new_id("call");
        let tool_name = "subagent".to_string();
        let tool_input = serde_json::json!({
            "description": description,
            "prompt": prompt,
            "subagent_type": subagent_type,
            "model": model,
            "session_id": session_id,
            "command": "/subagent",
        });

        let message_id = {
            let mut agent_guard = agent.lock().await;
            match agent_guard.add_manual_tool_use(
                tool_call_id.clone(),
                tool_name.clone(),
                tool_input.clone(),
            ) {
                Ok(message_id) => message_id,
                Err(error) => {
                    let _ = tx.send(ServerEvent::Error {
                        id,
                        message: crate::util::format_error_chain(&error),
                        retry_after_secs: None,
                    });
                    return;
                }
            }
        };

        let _ = tx.send(ServerEvent::ToolStart {
            id: tool_call_id.clone(),
            name: tool_name.clone(),
        });
        let _ = tx.send(ServerEvent::ToolInput {
            id: Some(tool_call_id.clone()),
            delta: tool_input.to_string(),
        });
        let _ = tx.send(ServerEvent::ToolExec {
            id: tool_call_id.clone(),
            name: tool_name.clone(),
        });

        let (registry, session_id, working_dir) = {
            let agent_guard = agent.lock().await;
            (
                agent_guard.registry(),
                agent_guard.session_id().to_string(),
                agent_guard.working_dir().map(std::path::PathBuf::from),
            )
        };

        let ctx = crate::tool::ToolContext {
            session_id,
            message_id,
            tool_call_id: tool_call_id.clone(),
            working_dir,
            stdin_request_tx: None,
            graceful_shutdown_signal: None,
            execution_mode: crate::tool::ToolExecutionMode::Direct,
        };

        let started = Instant::now();
        let tool_name_for_exec = tool_name.clone();
        let result = match tokio::spawn(async move {
            registry.execute(&tool_name_for_exec, tool_input, ctx).await
        })
        .await
        {
            Ok(result) => result,
            Err(error) => Err(anyhow::anyhow!("Tool task panicked: {}", error)),
        };
        let duration_ms = started.elapsed().as_millis().min(u64::MAX as u128) as u64;

        match result {
            Ok(output) => {
                let output_text = output.output.clone();
                let _ = tx.send(ServerEvent::ToolDone {
                    id: tool_call_id.clone(),
                    name: tool_name,
                    output: output_text,
                    error: None,
                });
                let persist = {
                    let mut agent_guard = agent.lock().await;
                    agent_guard.add_manual_tool_result(tool_call_id, output, duration_ms)
                };
                if let Err(error) = persist {
                    let _ = tx.send(ServerEvent::Error {
                        id,
                        message: crate::util::format_error_chain(&error),
                        retry_after_secs: None,
                    });
                    return;
                }
                let _ = tx.send(ServerEvent::Done { id });
            }
            Err(error) => {
                let error_msg = format!("Error: {}", error);
                let _ = tx.send(ServerEvent::ToolDone {
                    id: tool_call_id.clone(),
                    name: tool_name,
                    output: error_msg.clone(),
                    error: Some(error_msg.clone()),
                });
                let persist = {
                    let mut agent_guard = agent.lock().await;
                    agent_guard.add_manual_tool_error(tool_call_id, error_msg, duration_ms)
                };
                if let Err(persist_error) = persist {
                    let _ = tx.send(ServerEvent::Error {
                        id,
                        message: crate::util::format_error_chain(&persist_error),
                        retry_after_secs: None,
                    });
                    return;
                }
                let _ = tx.send(ServerEvent::Done { id });
            }
        }
    });
}
