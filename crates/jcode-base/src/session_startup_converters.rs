// Startup-stub and remote-snapshot session converters, extracted from
// `session.rs` to keep that file inside its size budget. Included inside the
// `Session` module, so private fields and helpers stay in scope.

impl Session {
    fn session_from_startup_stub(stub: SessionStartupStub) -> Self {
        let mut session = Self::create_with_id(stub.id, stub.parent_id, stub.title);
        session.custom_title = stub.custom_title;
        session.created_at = stub.created_at;
        session.updated_at = stub.updated_at;
        session.compaction = stub.compaction;
        session.provider_session_id = stub.provider_session_id;
        session.provider_key = stub.provider_key;
        session.spawn_allowed_tools = stub.spawn_allowed_tools;
        session.model = stub.model;
        session.route_api_method = stub.route_api_method;
        session.reasoning_effort = stub.reasoning_effort;
        session.subagent_model = stub.subagent_model;
        session.improve_mode = stub.improve_mode;
        session.autoreview_enabled = stub.autoreview_enabled;
        session.autojudge_enabled = stub.autojudge_enabled;
        session.is_canary = stub.is_canary;
        session.testing_build = stub.testing_build;
        session.working_dir = stub.working_dir;
        session.short_name = stub.short_name;
        session.status = stub.status;
        session.last_pid = stub.last_pid;
        session.last_active_at = stub.last_active_at;
        session.is_debug = stub.is_debug;
        session.saved = stub.saved;
        session.save_label = stub.save_label;
        session.messages.clear();
        session.env_snapshots.clear();
        session.memory_injections.clear();
        session.replay_events.clear();
        session.rebuild_memory_profile_cache();
        session.reset_persist_state(true);
        session
    }

    fn session_from_remote_startup_snapshot(snapshot: RemoteStartupSessionSnapshot) -> Self {
        let mut session = Self::create_with_id(snapshot.id, snapshot.parent_id, snapshot.title);
        session.custom_title = snapshot.custom_title;
        session.created_at = snapshot.created_at;
        session.updated_at = snapshot.updated_at;
        session.messages = snapshot.messages;
        session.compaction = snapshot.compaction;
        session.provider_session_id = snapshot.provider_session_id;
        session.provider_key = snapshot.provider_key;
        session.spawn_allowed_tools = snapshot.spawn_allowed_tools;
        session.model = snapshot.model;
        session.route_api_method = snapshot.route_api_method;
        session.reasoning_effort = snapshot.reasoning_effort;
        session.subagent_model = snapshot.subagent_model;
        session.improve_mode = snapshot.improve_mode;
        session.autoreview_enabled = snapshot.autoreview_enabled;
        session.autojudge_enabled = snapshot.autojudge_enabled;
        session.is_canary = snapshot.is_canary;
        session.testing_build = snapshot.testing_build;
        session.working_dir = snapshot.working_dir;
        session.short_name = snapshot.short_name;
        session.status = snapshot.status;
        session.last_pid = snapshot.last_pid;
        session.last_active_at = snapshot.last_active_at;
        session.is_debug = snapshot.is_debug;
        session.saved = snapshot.saved;
        session.save_label = snapshot.save_label;
        session.replay_events.clear();
        session.env_snapshots.clear();
        session.memory_injections.clear();
        session.mark_memory_profile_dirty();
        session.reset_persist_state(true);
        session.reset_provider_messages_cache();
        session
    }
}
