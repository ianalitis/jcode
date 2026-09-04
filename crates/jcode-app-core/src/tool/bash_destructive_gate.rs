//! The destructive-command gate for the `bash` tool (issue #604).
//!
//! Kept in its own file so the policy seam is easy to find and review: this is
//! the only thing standing between a model's `rm -rf` and the user's data.

/// Apply the deterministic destructive-command gate, returning refusal text
/// when the command must not run as-issued.
///
/// Stage 1 is a pure blast-radius assessment. Stage 2 requires a persisted,
/// single-use user approval scoped to the session, working directory, and exact
/// command. Catastrophic targets (`/`, `$HOME`, credential stores, device nodes)
/// are denied outright. See issue #604.
pub(super) fn destructive_command_refusal(
    command: &str,
    justification: Option<&str>,
    approval_id: Option<&str>,
    ctx: &crate::tool::ToolContext,
) -> Option<String> {
    let risk_ctx = jcode_command_risk::RiskContext::from_env(ctx.working_dir.clone());
    let assessment = jcode_command_risk::assess(command, &risk_ctx);
    match jcode_command_risk::gate(&assessment) {
        jcode_command_risk::GateOutcome::Allow => None,
        jcode_command_risk::GateOutcome::Deny { reason } => {
            crate::logging::warn(&format!("[bash] denied destructive command: {command}"));
            Some(reason)
        }
        jcode_command_risk::GateOutcome::Reflect { prompt } => {
            let working_dir = ctx
                .working_dir
                .as_ref()
                .map(|path| path.display().to_string())
                .unwrap_or_default();
            let scope = format!(
                "bash-confirm-v1\0{}\0{}\0{}",
                ctx.session_id, working_dir, command
            );
            let request_id = crate::safety::scoped_request_id(&scope);
            let system = crate::safety::SafetySystem::new();

            if approval_id == Some(request_id.as_str()) {
                match system.consume_approved_decision(&request_id) {
                    Ok(true) => {
                        crate::logging::info(&format!(
                            "[bash] consumed user approval {request_id} for destructive command"
                        ));
                        return None;
                    }
                    Ok(false) => {}
                    Err(error) => {
                        return Some(format!(
                            "This command was not run because its approval could not be verified: {error}"
                        ));
                    }
                }
            }

            let explanation = assessment.explanation();
            let rationale = justification
                .map(str::trim)
                .filter(|text| !text.is_empty())
                .unwrap_or(explanation.trim());
            let description = if working_dir.is_empty() {
                "Run a destructive Bash command".to_string()
            } else {
                format!("Run a destructive Bash command in {working_dir}")
            };
            let approval_mismatch = approval_id
                .filter(|supplied| *supplied != request_id)
                .map(|_| {
                    "The supplied approval_id does not match this session, working directory, and exact command.\n\n"
                })
                .unwrap_or("");

            let permission_result = system.request_permission(crate::safety::PermissionRequest {
                id: request_id.clone(),
                action: "bash_destructive_command".to_string(),
                description: description.clone(),
                rationale: rationale.to_string(),
                urgency: crate::safety::Urgency::High,
                wait: false,
                created_at: chrono::Utc::now(),
                context: Some(serde_json::json!({
                    "session_id": ctx.session_id,
                    "message_id": ctx.message_id,
                    "tool_call_id": ctx.tool_call_id,
                    "working_dir": ctx.working_dir.as_ref().map(|path| path.display().to_string()),
                    "review": {
                        "summary": description,
                        "why_permission_needed": rationale,
                        "requested_action": "bash",
                        "commands": [command],
                        "risks": [explanation.trim()],
                    }
                })),
            });
            match permission_result {
                crate::safety::PermissionResult::Queued {
                    request_id: queued_id,
                } if queued_id == request_id => {}
                crate::safety::PermissionResult::Denied { reason } => {
                    return Some(format!(
                        "This command was not run because its approval request could not be queued: {}",
                        reason.unwrap_or_else(|| "unknown persistence error".to_string())
                    ));
                }
                _ => {
                    return Some(
                        "This command was not run because its approval request returned an unexpected result."
                            .to_string(),
                    );
                }
            }

            crate::logging::info(&format!(
                "[bash] destructive command awaiting user approval {request_id}: {command}"
            ));
            Some(format!(
                "{prompt}\n\n{approval_mismatch}Permission request `{request_id}` is queued. Review it with `jcode permissions`. After approval, re-issue the exact same command with `\"approval_id\": \"{request_id}\"`. The approval is single-use. A model-supplied `justification` can explain the request but cannot authorize execution."
            ))
        }
    }
}

/// The `bash` tool's JSON schema, including the `justification` field the
/// destructive-command gate consumes.
///
/// Lives beside the gate so the schema and the policy that reads it stay in
/// sync, and so bash.rs stays inside the code-size budget.
pub(super) fn bash_parameters_schema() -> serde_json::Value {
    let cmd_desc = if cfg!(windows) {
        "The Windows command to execute via cmd.exe. Use cmd.exe syntax and quoting, not Bash syntax."
    } else {
        "The bash command to execute. Put large temp files under `$JCODE_SCRATCH_DIR`, not `/tmp`."
    };
    serde_json::json!({
        "type": "object",
        "required": ["command"],
        "properties": {
            "intent": crate::tool::intent_schema_property(),
            "command": {
                "type": "string",
                "description": cmd_desc
            },
            "timeout": {
                "type": "integer",
                "description": "Timeout in MILLISECONDS (not seconds), e.g. 600000 = 10min; kills with exit 124. Omit for no timeout."
            },
            "run_in_background": {
                "type": "boolean",
                "description": "Run in background. Emit `JCODE_PROGRESS {json}` lines for progress reporting."
            },
            "notify": {
                "type": "boolean",
                "description": "Notify on completion."
            },
            "wake": {
                "type": "boolean",
                "description": "Wake on completion."
            },
            "stall_wake_seconds": {
                "type": "integer",
                "description": "With run_in_background: wake the agent after this many seconds of no output/progress (min 30, resets on activity). Use for long jobs that may hang silently."
            },
            "justification": {
                "type": "string",
                "description": "Optional rationale shown to the user when a destructive command needs approval. This does not authorize execution."
            },
            "approval_id": {
                "type": "string",
                "description": "Single-use request ID from `jcode permissions`, valid only for the same session, working directory, and exact command."
            }
        }
    })
}
