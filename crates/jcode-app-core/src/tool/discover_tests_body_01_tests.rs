#[test]
fn details_executes_through_public_tool_interface() {
    let _guard = crate::storage::lock_test_env();
    // These cases exercise the real network path against a local server, so
    // opt out of any ambient telemetry opt-out inherited from the developer env.
    let _no_telemetry = RemovedEnvVar::new("JCODE_NO_TELEMETRY");
    let _do_not_track = RemovedEnvVar::new("DO_NOT_TRACK");
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("build current-thread test runtime")
        .block_on(async {
            let prev_home = std::env::var_os("JCODE_HOME");
            let temp = tempfile::tempdir().unwrap();
            crate::env::set_var("JCODE_HOME", temp.path());
            let body = json!({
                "tool": "Stripe",
                "fit": "strong",
                "summary": "Metered billing and webhook reconciliation are supported.",
                "capabilities": ["Usage meters", "Invoice webhooks"],
                "sources": [{
                    "title": "Usage billing",
                    "provider_url": "https://docs.stripe.com/billing/subscriptions/usage-based",
                    "cached_url": "https://jcode.sh/docs/stripe/usage-based"
                }],
                "next_action": "select"
            })
            .to_string();
            let (endpoint, server) = one_shot_server("HTTP/1.1 200 OK", body).await;
            std::fs::write(
                temp.path().join("config.toml"),
                format!("[sponsors]\nenabled = true\nendpoint = \"{endpoint}\"\n"),
            )
            .unwrap();
            crate::config::Config::invalidate_cache();

            let output = DiscoverToolsTool::new().execute(json!({
                    "action": "details",
                    "category": "payments",
                    "query": "confirm metered subscription billing and webhook reconciliation support",
                    "reason": "the current SaaS workflow requires usage reporting and reliable invoice state updates",
                    "tool": "Stripe",
                    "work_relevance": "core_requirement",
                    "investigation_goal": "capability_fit",
                    "requirements": ["Webhook status updates"],
                    "topics": ["capabilities", "limitations"]
                }), test_ctx()).await.unwrap();

            assert_eq!(output.title.as_deref(), Some("stripe details"));
            assert!(output.output.contains("Fit: strong"));
            assert!(output.output.contains("Usage meters"));
            assert!(
                output
                    .output
                    .contains("https://docs.stripe.com/billing/subscriptions/usage-based")
            );
            assert!(
                output
                    .output
                    .contains("Jcode snapshot: https://jcode.sh/docs/stripe/usage-based")
            );
            let metadata = output.metadata.unwrap();
            assert_eq!(metadata["integration_details"], true);
            assert_eq!(metadata["work_relevance"], "core_requirement");
            assert_eq!(metadata["investigation_goal"], "capability_fit");
            let request = server.await.unwrap();
            assert!(request.starts_with("POST /details HTTP/1.1"), "{request}");
            assert!(
                request.contains("\"work_relevance\":\"core_requirement\""),
                "{request}"
            );

            if let Some(prev) = prev_home {
                crate::env::set_var("JCODE_HOME", prev);
            } else {
                crate::env::remove_var("JCODE_HOME");
            }
            crate::config::Config::invalidate_cache();
        });
}

#[test]
fn git_category_executes_end_to_end_with_enabled_config_and_local_server() {
    let _guard = crate::storage::lock_test_env();
    // These cases exercise the real network path against a local server, so
    // opt out of any ambient telemetry opt-out inherited from the developer env.
    let _no_telemetry = RemovedEnvVar::new("JCODE_NO_TELEMETRY");
    let _do_not_track = RemovedEnvVar::new("DO_NOT_TRACK");
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("build current-thread test runtime")
        .block_on(async {
            let prev_home = std::env::var_os("JCODE_HOME");
            let temp = tempfile::tempdir().unwrap();
            crate::env::set_var("JCODE_HOME", temp.path());

            let body = json!({"tools": [{"name": "github", "blurb": "repository hosting and collaboration", "url": "https://github.com", "setup": "MCP server: npx github-mcp"}]}).to_string();
            let (endpoint, server) = one_shot_server("HTTP/1.1 200 OK", body).await;
            std::fs::write(
                temp.path().join("config.toml"),
                format!("[sponsors]\nenabled = true\nendpoint = \"{endpoint}\"\n"),
            )
            .unwrap();
            crate::config::Config::invalidate_cache();

            let tool = DiscoverToolsTool::new();
            let output = tool
                    .execute(
                        json!({
                            "category": "git",
                            "query": "host and collaborate on git repositories",
                            "reason": "task requires remote repository collaboration capabilities not present in the current tools"
                        }),
                        test_ctx(),
                    )
                    .await
                    .unwrap();

            assert!(output.output.contains("github"));
            assert!(output.output.contains("Jcode integration directory"));
            assert!(
                output
                    .output
                    .contains("recommendations must be based only on fit")
            );
            // End to end, not just in render_listing: a browse must never hand the
            // agent runnable setup, or it has no reason to call select.
            assert!(
                !output.output.contains("npx github-mcp"),
                "browse leaked setup instructions: {}",
                output.output
            );
            assert!(output.output.contains("action `select`"));
            let title = output.title.unwrap();
            assert_eq!(title, "git", "{title}");
            let meta = output.metadata.unwrap();
            assert_eq!(meta["sponsored_discovery"], true);

            let request = server.await.unwrap();
            assert!(request.contains("category=git"), "{request}");

            // Opted-out config: execute refuses without any network call.
            std::fs::write(
                temp.path().join("config.toml"),
                "[sponsors]\nenabled = false\n",
            )
            .unwrap();
            crate::config::Config::invalidate_cache();
            let err = tool
                .execute(json!({"category": "payments", "reason": "x"}), test_ctx())
                .await
                .unwrap_err();
            assert!(err.to_string().contains("disabled"));

            if let Some(prev) = prev_home {
                crate::env::set_var("JCODE_HOME", prev);
            } else {
                crate::env::remove_var("JCODE_HOME");
            }
            crate::config::Config::invalidate_cache();
        });
}
