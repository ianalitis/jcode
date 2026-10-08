use super::CloudSessionListItem;

fn html_escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

fn message_count_label(value: Option<&serde_json::Value>) -> String {
    match value {
        Some(serde_json::Value::Number(num)) => num.to_string(),
        Some(serde_json::Value::String(text)) => text.clone(),
        _ => "-".to_string(),
    }
}

pub(super) fn render_cloud_sessions_dashboard_html(
    user_id: &str,
    items: &[CloudSessionListItem],
    view_links: &std::collections::BTreeMap<String, String>,
) -> String {
    let generated = chrono::Utc::now().to_rfc3339();
    let mut rows = String::new();
    for item in items {
        let session_id = item.session_id.as_deref().unwrap_or("(unknown)");
        let title = item
            .title
            .as_deref()
            .filter(|value| !value.is_empty())
            .or(item.short_name.as_deref())
            .unwrap_or("(untitled)");
        let uploaded = item.uploaded_at.as_deref().unwrap_or("-");
        // When a local per-session viewer was generated, link the session id to it.
        let id_cell = match item.session_id.as_deref().and_then(|id| view_links.get(id)) {
            Some(link) => format!(
                "<a href='{}'>{}</a>",
                html_escape(link),
                html_escape(session_id)
            ),
            None => html_escape(session_id),
        };
        rows.push_str(&format!(
            "<tr><td class='id'>{}</td><td>{}</td><td class='num'>{}</td><td class='ts'>{}</td></tr>\n",
            id_cell,
            html_escape(title),
            html_escape(&message_count_label(item.message_count.as_ref())),
            html_escape(uploaded),
        ));
    }
    if rows.is_empty() {
        rows.push_str("<tr><td colspan='4' class='empty'>No uploaded sessions found.</td></tr>\n");
    }
    format!(
        "<!doctype html><meta charset='utf-8'>\n\
<title>Jade Cloud Sessions Dashboard</title>\n\
<style>body{{font-family:system-ui,sans-serif;max-width:1100px;margin:2rem auto;padding:0 1rem;color:#1b1b1f}}\
h1{{margin-bottom:0.2rem}}.meta{{color:#666;margin-bottom:1.5rem}}\
table{{border-collapse:collapse;width:100%}}th,td{{text-align:left;padding:0.5rem 0.6rem;border-bottom:1px solid #e3e3e8}}\
th{{background:#f6f8fa;position:sticky;top:0}}td.id{{font-family:ui-monospace,monospace;font-size:0.85rem}}\
td.id a{{color:#0a58ca;text-decoration:none}}td.id a:hover{{text-decoration:underline}}\
td.num{{text-align:right}}td.ts{{white-space:nowrap;color:#555}}td.empty{{text-align:center;color:#888;padding:2rem}}\
tr:hover td{{background:#fafbff}}</style>\n\
<h1>Jade Cloud Sessions</h1>\n\
<div class='meta'>user: {user} &middot; {count} session(s) &middot; generated {generated}</div>\n\
<table><thead><tr><th>Session ID</th><th>Title</th><th>Messages</th><th>Uploaded</th></tr></thead>\n\
<tbody>\n{rows}</tbody></table>\n",
        user = html_escape(user_id),
        count = items.len(),
        generated = html_escape(&generated),
        rows = rows,
    )
}
