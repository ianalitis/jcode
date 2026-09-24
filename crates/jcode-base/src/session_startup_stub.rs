#[derive(Debug, Deserialize)]
struct SessionStartupStub {
    id: String,
    #[serde(default)]
    parent_id: Option<String>,
    #[serde(default)]
    title: Option<String>,
    #[serde(default)]
    custom_title: Option<String>,
    #[serde(default)]
    system_prompt: Option<String>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
    #[serde(default)]
    compaction: Option<StoredCompactionState>,
    #[serde(default)]
    provider_session_id: Option<String>,
    #[serde(default)]
    provider_key: Option<String>,
    #[serde(default)]
    spawn_allowed_tools: Option<Vec<String>>,
    #[serde(default)]
    model: Option<String>,
    #[serde(default)]
    route_api_method: Option<String>,
    #[serde(default)]
    reasoning_effort: Option<String>,
    #[serde(default)]
    subagent_model: Option<String>,
    #[serde(default)]
    improve_mode: Option<SessionImproveMode>,
    #[serde(default)]
    autoreview_enabled: Option<bool>,
    #[serde(default)]
    autojudge_enabled: Option<bool>,
    #[serde(default)]
    is_canary: bool,
    #[serde(default)]
    testing_build: Option<String>,
    #[serde(default)]
    working_dir: Option<String>,
    #[serde(default)]
    short_name: Option<String>,
    #[serde(default)]
    status: SessionStatus,
    #[serde(default)]
    last_pid: Option<u32>,
    #[serde(default)]
    last_active_at: Option<DateTime<Utc>>,
    #[serde(default)]
    is_debug: bool,
    #[serde(default)]
    saved: bool,
    #[serde(default)]
    save_label: Option<String>,
}
