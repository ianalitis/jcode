/// Locate a Cursor agent transcript file for the given session id.
///
/// Cursor stores transcripts at
/// `~/.cursor/projects/<project>/agent-transcripts/<session-id>/<session-id>.jsonl`,
/// so the session id is the file stem. We scan the project tree for a matching
/// stem rather than guessing the project dir.
fn find_cursor_session_file(session_id: &str) -> Result<PathBuf> {
    let root = crate::storage::user_home_path(".cursor/projects")?;
    for path in collect_files_recursive(&root, "jsonl") {
        if cursor_session_id_from_path(&path) == session_id {
            return Ok(path);
        }
    }
    anyhow::bail!("Cursor session {} not found", session_id)
}

pub fn import_cursor_session(session_id: &str) -> Result<Session> {
    let path = find_cursor_session_file(session_id)?;
    import_cursor_session_from_path(&path, Some(session_id))
}

pub fn import_cursor_session_from_path(
    session_path: &Path,
    session_id_hint: Option<&str>,
) -> Result<Session> {
    let session_id = session_id_hint
        .map(|id| id.to_string())
        .unwrap_or_else(|| cursor_session_id_from_path(session_path));
    let created_at =
        jcode_import_core::file_modified_datetime(session_path).unwrap_or_else(Utc::now);

    let mut session = Session::create_with_id(imported_cursor_session_id(&session_id), None, None);
    session.provider_session_id = Some(session_id.clone());
    session.provider_key = Some("cursor".to_string());
    session.working_dir = cursor_cwd_from_transcript_path(session_path);

    let file = File::open(session_path)?;
    let reader = BufReader::new(file);
    let mut title: Option<String> = None;
    for line in reader.lines() {
        let line = line?;
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let Ok(value) = serde_json::from_str::<serde_json::Value>(trimmed) else {
            continue;
        };
        let role = match value
            .get("role")
            .and_then(|v| v.as_str())
            .unwrap_or_default()
        {
            "user" | "human" => Role::User,
            "assistant" | "model" => Role::Assistant,
            _ => continue,
        };
        let content = value
            .get("message")
            .and_then(|message| message.get("content"))
            .or_else(|| value.get("content"))
            .cloned()
            .unwrap_or(serde_json::Value::Null);
        let text = extract_external_text_from_json(&content, true);
        if text.trim().is_empty() {
            continue;
        }
        if title.is_none() && role == Role::User {
            title = Some(truncate_title_text(&text, 72));
        }
        append_text_message(&mut session, role, text, None);
    }

    session.title = title.or_else(|| Some(format!("Cursor session {}", session_id)));
    finalize_imported_session(session, created_at, Some(created_at))
}
