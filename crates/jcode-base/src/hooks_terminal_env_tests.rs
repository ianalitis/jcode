#[cfg(unix)]
#[tokio::test]
async fn hook_process_replaces_daemon_terminal_env_with_client_snapshot() {
    let _guard = crate::storage::lock_test_env();
    let temp = tempfile::TempDir::new().expect("temporary directory");
    let script = write_executable_script(
        temp.path(),
        "env.sh",
        "#!/bin/sh\nprintf '%s|%s|%s|%s' \"$TMUX_PANE\" \"$HERDR_PANE_ID\" \"$JCODE_CLIENT_TMUX_PANE\" \"$JCODE_CLIENT_HERDR_PANE_ID\"\n",
    );
    let previous_tmux = std::env::var_os("TMUX_PANE");
    let previous_herdr = std::env::var_os("HERDR_PANE_ID");
    crate::env::set_var("TMUX_PANE", "daemon-pane");
    crate::env::set_var("HERDR_PANE_ID", "daemon-herdr");

    let run_for_pane = |tmux: &'static str, herdr: &'static str| {
        let script = script.clone();
        with_client_terminal_env(
            vec![
                ("TMUX_PANE".to_string(), tmux.to_string()),
                ("HERDR_PANE_ID".to_string(), herdr.to_string()),
            ],
            async move {
                tokio::task::yield_now().await;
                build_hook_process(&script.to_string_lossy(), &HookEvent::new("turn_start"))
                    .expect("hook command")
                    .output()
                    .expect("run hook")
            },
        )
    };
    let (first_output, second_output) = tokio::join!(
        run_for_pane("client-pane-a", "herdr-pane-a"),
        run_for_pane("client-pane-b", "herdr-pane-b")
    );

    match previous_tmux {
        Some(value) => crate::env::set_var("TMUX_PANE", value),
        None => crate::env::remove_var("TMUX_PANE"),
    }
    match previous_herdr {
        Some(value) => crate::env::set_var("HERDR_PANE_ID", value),
        None => crate::env::remove_var("HERDR_PANE_ID"),
    }
    assert!(first_output.status.success());
    assert!(second_output.status.success());
    assert_eq!(
        String::from_utf8_lossy(&first_output.stdout),
        "client-pane-a|herdr-pane-a|client-pane-a|herdr-pane-a"
    );
    assert_eq!(
        String::from_utf8_lossy(&second_output.stdout),
        "client-pane-b|herdr-pane-b|client-pane-b|herdr-pane-b"
    );
}
