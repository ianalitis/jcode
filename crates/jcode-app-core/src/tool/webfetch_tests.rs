use super::*;

#[test]
fn research_evidence_schema_is_opt_in_and_retrievable() {
    let schema = WebFetchTool::new().parameters_schema();
    assert_eq!(schema["properties"]["retain_evidence"]["default"], false);
    assert!(
        schema["properties"]["url"]["description"]
            .as_str()
            .unwrap()
            .contains("evidence:")
    );
}

#[test]
fn public_ip_policy_blocks_special_address_space() {
    for ip in [
        "0.0.0.0",
        "10.0.0.1",
        "100.64.0.1",
        "127.0.0.1",
        "169.254.169.254",
        "172.16.0.1",
        "192.0.0.1",
        "192.0.2.1",
        "192.168.0.1",
        "198.18.0.1",
        "198.51.100.1",
        "203.0.113.1",
        "224.0.0.1",
        "255.255.255.255",
    ] {
        assert!(!is_public_ipv4(ip.parse().unwrap()), "allowed {ip}");
    }
    for ip in [
        "::",
        "::1",
        "::ffff:127.0.0.1",
        "fc00::1",
        "fe80::1",
        "2001:db8::1",
        "2002::1",
        "3fff::1",
        "ff02::1",
    ] {
        assert!(!is_public_ipv6(ip.parse().unwrap()), "allowed {ip}");
    }

    assert!(is_public_ipv4("1.1.1.1".parse().unwrap()));
    assert!(is_public_ipv4("8.8.8.8".parse().unwrap()));
    assert!(is_public_ipv6("2001:4860:4860::8888".parse().unwrap()));
    assert!(is_public_ipv6("2606:4700:4700::1111".parse().unwrap()));
    assert!(is_public_ipv6("::ffff:8.8.8.8".parse().unwrap()));
}

#[tokio::test]
async fn resolved_target_rejects_literal_and_dns_local_addresses() {
    for url in [
        "http://127.0.0.1/",
        "http://169.254.169.254/latest/meta-data/",
        "http://[::1]/",
        "http://[::ffff:127.0.0.1]/",
        "file:///etc/passwd",
    ] {
        let err = ResolvedTarget::resolve(Url::parse(url).unwrap())
            .await
            .unwrap_err();
        assert!(
            err.to_string().contains("non-public") || err.to_string().contains("must use http"),
            "unexpected error for {url}: {err}"
        );
    }

    let err = ResolvedTarget::resolve(Url::parse("http://localhost/").unwrap())
        .await
        .unwrap_err();
    assert!(err.to_string().contains("non-public"), "{err}");
}

#[tokio::test]
async fn redirect_targets_are_revalidated() {
    let base = Url::parse("https://example.com/start").unwrap();
    let target = redirect_url(&base, "http://127.0.0.1/admin").unwrap();
    let err = ResolvedTarget::resolve(target).await.unwrap_err();
    assert!(err.to_string().contains("non-public"), "{err}");

    assert_eq!(
        redirect_url(&base, "/next").unwrap().as_str(),
        "https://example.com/next"
    );
}

#[test]
fn redirect_chain_uses_one_deadline() {
    let start = tokio::time::Instant::now();
    let deadline = start + Duration::from_secs(30);
    assert_eq!(
        remaining_timeout(deadline, start).unwrap(),
        Duration::from_secs(30)
    );
    assert_eq!(
        remaining_timeout(deadline, start + Duration::from_secs(12)).unwrap(),
        Duration::from_secs(18)
    );
    assert!(remaining_timeout(deadline, deadline).is_err());
}

#[test]
fn strips_non_prose_elements() {
    let html = "<nav><a href='/x'>Menu</a></nav><p>Body text</p>\
                <aside>Related</aside><form><select><option>Pick</option></select></form>";
    let md = html_to_markdown(html);
    assert!(md.contains("Body text"));
    assert!(!md.contains("Menu"), "nav should be dropped: {md}");
    assert!(!md.contains("Related"), "aside should be dropped: {md}");
    assert!(
        !md.contains("Pick"),
        "form controls should be dropped: {md}"
    );
}

#[test]
fn keeps_article_header_and_footer_content() {
    // <header> usually holds the title/byline and <footer> can hold
    // article attribution, so neither is treated as chrome.
    let html = "<article><header><h1>Real Title</h1><p>By Author</p></header>\
                <p>Body</p><footer>Published 2026</footer></article>";
    let md = html_to_markdown(html);
    for needle in ["Real Title", "By Author", "Body", "Published 2026"] {
        assert!(md.contains(needle), "{needle} missing from {md}");
    }
}

#[test]
fn drops_empty_links_and_overlong_targets() {
    assert_eq!(render_link("https://example.com", ""), "");
    assert_eq!(render_link("#section", "Jump"), "Jump");
    let long = format!("https://example.com/?code={}", "a".repeat(MAX_URL_CHARS));
    assert_eq!(render_link(&long, "Run"), "Run");
    assert_eq!(
        render_link("https://example.com", "Home"),
        "[Home](https://example.com)"
    );
}

#[test]
fn strips_html_comments() {
    let md = html_to_markdown("<p>Keep</p><!-- build:12345 drop me -->");
    assert!(md.contains("Keep"));
    assert!(!md.contains("drop me"), "comment retained: {md}");
}

#[test]
fn does_not_leak_attributes_containing_angle_brackets() {
    // Parsoid-style tags embed JSON in attributes; a naive `<[^>]+>` regex
    // stops at the first `>` inside the value and dumps the rest as text.
    let html = r#"<span data-mw='{"wt":"[[a]] > [[b]]"}'>Visible</span>"#;
    let text = html_to_text(html);
    assert_eq!(text, "Visible");
}

#[test]
#[test]
fn truncation_note_names_the_spilled_response() {
    let spilled = super::truncation_note(
        90_000,
        Some(std::path::Path::new("/tmp/tool-output-webfetch.txt")),
    );
    assert!(spilled.contains("40000 of 90000 chars"), "{spilled}");
    assert!(
        spilled.contains("full response saved at /tmp/tool-output-webfetch.txt"),
        "{spilled}"
    );
    assert!(spilled.contains("offset/limit"), "{spilled}");

    let fallback = super::truncation_note(90_000, None);
    assert!(fallback.contains("40000 of 90000 chars"), "{fallback}");
    assert!(
        !fallback.contains("saved at"),
        "without a spill the note must not point at a file: {fallback}"
    );
    assert!(fallback.contains("more specific URL"), "{fallback}");
}

#[test]
fn caps_output_length() {
    let long = "line of text\n".repeat(MAX_OUTPUT_CHARS);
    let (out, truncated) = truncate_output(long);
    assert!(truncated);
    assert!(out.len() <= MAX_OUTPUT_CHARS);
}

#[test]
fn keeps_short_output_intact() {
    let (out, truncated) = truncate_output("hello".to_string());
    assert!(!truncated);
    assert_eq!(out, "hello");
}

#[test]
fn truncation_respects_char_boundaries() {
    // Multi-byte chars straddling the cut must not panic or corrupt output.
    let long = "é".repeat(MAX_OUTPUT_CHARS);
    let (out, truncated) = truncate_output(long);
    assert!(truncated);
    assert!(out.chars().all(|c| c == 'é'));
}
