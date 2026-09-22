use super::*;

struct ResolvedSearchScope {
    root: Option<String>,
    glob: Option<String>,
}

fn resolved_search_scope(
    ctx: &ToolContext,
    path: Option<&str>,
    file: Option<&str>,
    glob: Option<&str>,
) -> ResolvedSearchScope {
    // `file` scopes grep/find to one exact file when `path` is absent.
    let path = path.or(file);
    let Some(path) = path else {
        return ResolvedSearchScope {
            root: None,
            glob: normalized_agentgrep_glob_owned(glob),
        };
    };

    let resolved = resolve_path_arg(ctx, path);
    if resolved.is_file() {
        let root = resolved
            .parent()
            .unwrap_or_else(|| Path::new("."))
            .display()
            .to_string();
        let glob = resolved
            .file_name()
            .map(|name| name.to_string_lossy().into_owned());
        return ResolvedSearchScope {
            root: Some(root),
            glob,
        };
    }

    ResolvedSearchScope {
        root: Some(resolved.display().to_string()),
        glob: normalized_agentgrep_glob_owned(glob),
    }
}

pub(super) fn build_grep_args(params: &AgentGrepInput, ctx: &ToolContext) -> Result<GrepArgs> {
    let query = params.query.clone().ok_or_else(|| {
        anyhow::anyhow!(
            "agentgrep grep needs something to search for: pass `query` (the `pattern` alias works too). \
             To inspect a single file instead, pass mode=\"outline\" with `file`."
        )
    })?;
    let scope = resolved_search_scope(
        ctx,
        params.path.as_deref(),
        params.file.as_deref(),
        params.glob.as_deref(),
    );
    Ok(GrepArgs {
        query,
        regex: params.regex.unwrap_or(false),
        file_type: params.file_type.clone(),
        json: false,
        paths_only: params.paths_only.unwrap_or(false),
        hidden: params.hidden.unwrap_or(false),
        no_ignore: params.no_ignore.unwrap_or(false),
        path: scope.root,
        glob: scope.glob,
        // Follow symlinks, matching prior agentgrep behavior and `rg`'s
        // default. Repositories legitimately symlink shared crates and
        // vendored sources, and silently skipping them would make search
        // results quietly incomplete.
        no_follow: false,
    })
}

pub(super) fn build_find_args(params: &AgentGrepInput, ctx: &ToolContext) -> Result<FindArgs> {
    let query = params.query.as_deref().unwrap_or_default();
    if query.trim().is_empty()
        && params.path.as_deref().is_none_or(str::is_empty)
        && params.file.as_deref().is_none_or(str::is_empty)
        && normalized_agentgrep_glob(params.glob.as_deref()).is_none()
        && params.file_type.as_deref().is_none_or(str::is_empty)
    {
        return Err(anyhow::anyhow!(
            "agentgrep find requires 'query' unless path, glob, or type narrows the search"
        ));
    }
    let scope = resolved_search_scope(
        ctx,
        params.path.as_deref(),
        params.file.as_deref(),
        params.glob.as_deref(),
    );
    Ok(FindArgs {
        query_parts: query.split_whitespace().map(ToOwned::to_owned).collect(),
        file_type: params.file_type.clone(),
        json: false,
        paths_only: params.paths_only.unwrap_or(false),
        debug_score: params.debug_score.unwrap_or(false),
        max_files: params.max_files.unwrap_or(10),
        hidden: params.hidden.unwrap_or(false),
        no_ignore: params.no_ignore.unwrap_or(false),
        path: scope.root,
        glob: scope.glob,
        // Follow symlinks, matching prior agentgrep behavior and `rg`'s
        // default. Repositories legitimately symlink shared crates and
        // vendored sources, and silently skipping them would make search
        // results quietly incomplete.
        no_follow: false,
    })
}

pub(super) fn build_outline_args(
    params: &AgentGrepInput,
    ctx: &ToolContext,
    context_json_path: Option<&Path>,
) -> Result<OutlineArgs> {
    // Agents sometimes point `path` at the file itself, either instead of or
    // in addition to `file`. Treat a file-valued `path` as the outline target
    // so the file argument is not joined onto it (for example,
    // ".../todo.rs/.../todo.rs").
    if let Some(path) = params.path.as_deref() {
        let resolved = resolve_path_arg(ctx, path);
        if resolved.is_file() {
            return Ok(OutlineArgs {
                file: resolved.display().to_string(),
                json: false,
                max_items: None,
                path: None,
                context_json: context_json_path.map(|path| path.display().to_string()),
            });
        }
    }

    let file = outline_file_arg(params)?;
    Ok(OutlineArgs {
        file,
        json: false,
        max_items: None,
        path: resolved_root_string(ctx, params.path.as_deref()),
        context_json: context_json_path.map(|path| path.display().to_string()),
    })
}

pub(super) fn build_smart_args_and_query(
    params: &AgentGrepInput,
    ctx: &ToolContext,
    context_json_path: Option<&Path>,
) -> Result<(SmartArgs, SmartQuery, Option<String>)> {
    let (terms, dedupe_note) = dedupe_smart_dsl_keys(&trace_or_smart_terms_owned(params)?);
    let query = parse_smart_query(&terms).map_err(|err| {
        anyhow::anyhow!(
            "{}\n\ntrace queries use a small DSL. Example:\n  agentgrep trace subject:auth_status relation:rendered support:ui",
            err
        )
    })?;
    let scope = resolved_search_scope(
        ctx,
        params.path.as_deref(),
        params.file.as_deref(),
        params.glob.as_deref(),
    );

    let args = SmartArgs {
        terms,
        json: false,
        max_files: params.max_files.unwrap_or(5),
        max_regions: params.max_regions.unwrap_or(6),
        full_region: parse_full_region_mode(params.full_region.as_deref())?,
        debug_plan: params.debug_plan.unwrap_or(false),
        debug_score: params.debug_score.unwrap_or(false),
        paths_only: params.paths_only.unwrap_or(false),
        path: scope.root,
        file_type: params.file_type.clone(),
        glob: scope.glob,
        hidden: params.hidden.unwrap_or(false),
        no_ignore: params.no_ignore.unwrap_or(false),
        context_json: context_json_path.map(|path| path.display().to_string()),
    };

    Ok((args, query, dedupe_note))
}

pub(super) fn trace_or_smart_terms_owned(params: &AgentGrepInput) -> Result<Vec<String>> {
    if let Some(terms) = params.terms.as_ref().filter(|terms| !terms.is_empty()) {
        return Ok(terms.clone());
    }

    if params.mode == "smart"
        && let Some(query) = params.query.as_deref()
    {
        let split_terms: Vec<String> = query
            .split_whitespace()
            .filter(|term| !term.is_empty())
            .map(ToOwned::to_owned)
            .collect();
        if !split_terms.is_empty() {
            return Ok(split_terms);
        }
    }

    let field_hint = if params.mode == "smart" {
        "non-empty 'terms' or 'query'"
    } else {
        "non-empty 'terms'"
    };

    Err(anyhow::anyhow!(
        "agentgrep {} requires {}",
        params.mode,
        field_hint
    ))
}

fn outline_file_arg(params: &AgentGrepInput) -> Result<String> {
    params
        .file
        .clone()
        .or_else(|| params.query.clone())
        .or_else(|| {
            params
                .terms
                .as_ref()
                .and_then(|terms| terms.first().cloned())
        })
        .ok_or_else(|| {
            anyhow::anyhow!("agentgrep outline requires 'file' (or legacy 'query' / first term)")
        })
}

fn parse_full_region_mode(value: Option<&str>) -> Result<FullRegionMode> {
    match value.unwrap_or("auto").trim().to_ascii_lowercase().as_str() {
        "auto" => Ok(FullRegionMode::Auto),
        "always" => Ok(FullRegionMode::Always),
        "never" => Ok(FullRegionMode::Never),
        other => Err(anyhow::anyhow!(
            "agentgrep trace full_region must be one of: auto, always, never; got {other}"
        )),
    }
}

fn resolved_root_string(ctx: &ToolContext, path: Option<&str>) -> Option<String> {
    path.map(|path| resolve_path_arg(ctx, path).display().to_string())
}

pub(super) fn resolve_search_root(ctx: &ToolContext, path: Option<&str>) -> Result<PathBuf> {
    path.map(PathBuf::from)
        .or_else(|| ctx.working_dir.clone())
        .ok_or_else(|| anyhow::anyhow!("agentgrep requires a session working directory"))
}

pub(super) fn summarize_agentgrep_request(
    params: &AgentGrepInput,
    ctx: &ToolContext,
    context_json_path: Option<&Path>,
) -> String {
    let mut parts = vec![format!("mode={}", params.mode)];
    if let Some(query) = params.query.as_deref() {
        parts.push(format!("query={}", util::truncate_str(query, 80)));
    }
    if let Some(file) = params.file.as_deref() {
        parts.push(format!("file={file}"));
    }
    if let Some(terms) = params.terms.as_ref() {
        parts.push(format!(
            "terms={}",
            util::truncate_str(&terms.join(" "), 80)
        ));
    }
    if let Some(path) = resolved_root_string(ctx, params.path.as_deref()) {
        parts.push(format!("root={path}"));
    }
    if let Some(glob) = normalized_agentgrep_glob(params.glob.as_deref()) {
        parts.push(format!("glob={glob}"));
    }
    if let Some(file_type) = params.file_type.as_deref() {
        parts.push(format!("type={file_type}"));
    }
    if params.paths_only.unwrap_or(false) {
        parts.push("paths_only=true".to_string());
    }
    if context_json_path.is_some() {
        parts.push("context_json=true".to_string());
    }
    parts.join(" ")
}

/// Turn a query that cannot compile as a regex into a literal one, reporting it.
///
/// Upstream reports `invalid regex` and stops. The observed case is a caller passing
/// source text with `regex=true` - `fn(` from a Rust snippet - where the intent is
/// plainly a literal search; upstream's own error hint says to drop `regex=true`. Doing
/// that automatically removes a failed turn, and the returned note keeps the
/// substitution visible instead of silently changing what was searched.
///
/// Compilation mirrors upstream exactly (`regex::Regex::new`), so a pattern accepted
/// here is accepted there.
pub(super) fn degrade_uncompilable_regex(args: &mut GrepArgs) -> Option<String> {
    if !args.regex {
        return None;
    }
    let error = match regex::Regex::new(&args.query) {
        Ok(_) => return None,
        Err(error) => error,
    };
    args.regex = false;
    // `regex::Error` renders a multi-line caret diagram; the last line carries the
    // reason ("error: unclosed group"), and that is the part worth reporting.
    let rendered = error.to_string();
    let reason = rendered
        .lines()
        .last()
        .unwrap_or(&rendered)
        .trim()
        .trim_start_matches("error:")
        .trim()
        .to_string();
    Some(format!(
        "note: query is not a valid regex ({reason}); searched literally instead"
    ))
}

/// Keep the first occurrence of each trace DSL key and report the ones dropped.
///
/// `parse_smart_query` refuses a repeated key outright. A caller repeating `subject:`
/// is expressing something the DSL cannot honour rather than a mistake worth losing a
/// turn over, so the first value is kept and the dropped terms are named in the output,
/// which is enough for the caller to retry with the right shape. Bare terms have no key
/// to repeat and pass through untouched.
pub(super) fn dedupe_smart_dsl_keys(terms: &[String]) -> (Vec<String>, Option<String>) {
    let mut seen: HashSet<String> = HashSet::new();
    let mut kept = Vec::with_capacity(terms.len());
    let mut dropped = Vec::new();
    for term in terms {
        // A term with no key cannot repeat a key, so it always passes through.
        let Some((key, _)) = term.split_once(':') else {
            kept.push(term.clone());
            continue;
        };
        let key = key.trim().to_string();
        if key.is_empty() {
            kept.push(term.clone());
            continue;
        }
        if seen.insert(key) {
            kept.push(term.clone());
        } else {
            dropped.push(term.clone());
        }
    }
    let note = (!dropped.is_empty()).then(|| {
        format!(
            "note: dropped repeated trace terms [{}]; the first value of each key is used",
            dropped.join(", ")
        )
    });
    (kept, note)
}

/// Infer `outline` when a caller names a single file and gives nothing to search for.
///
/// `mode` defaults to grep, and a call carrying only a file cannot grep. The measured
/// real case is a call with `file_path` and an intent of "outline the craft contract"
/// that omitted `mode` and failed with "requires 'query'". Inferring outline returns
/// what was asked for, and the note says so. A path that does not look like a file (no
/// extension in its last segment) is left alone, so a directory search still errors
/// with the query guidance rather than silently outlining nothing.
pub(super) fn infer_outline_for_file_only_call(
    params: &AgentGrepInput,
) -> Option<(String, String)> {
    if params.mode != "grep" {
        return None;
    }
    if params.query.is_some() || params.terms.as_ref().is_some_and(|terms| !terms.is_empty()) {
        return None;
    }
    let file = params.file.as_deref().or(params.path.as_deref())?;
    let last_segment = file.rsplit('/').next().unwrap_or(file);
    if !last_segment.contains('.') {
        return None;
    }
    Some((
        "outline".to_string(),
        format!("note: no query was given and {file} names a file; ran as outline"),
    ))
}
