use super::{OpenAIProvider, Provider};

impl OpenAIProvider {
    pub(super) fn normalize_reasoning_effort(raw: &str) -> Option<String> {
        let value = raw.trim().to_lowercase();
        if value.is_empty() {
            return None;
        }
        match value.as_str() {
            // `swarm` is a UI sentinel meaning "configured root effort + use the swarm tool".
            // We keep it stored so the UI/session reflect it and the agent injects
            // the swarm directive; it is translated to a real effort at request time
            // by `api_reasoning_effort`.
            "none" | "minimal" | "low" | "medium" | "high" | "xhigh" | "max" | "swarm"
            | "swarm-deep" => Some(value),
            other => {
                jcode_base::logging::info(&format!(
                    "Warning: Ignoring unsupported OpenAI reasoning effort '{}'; expected none|minimal|low|medium|high|xhigh|max.",
                    other
                ));
                None
            }
        }
    }

    /// Default reasoning effort to apply when the user has *not* explicitly
    /// configured one. GPT-5.6 Sol defaults to `low`: it is strong enough at
    /// low effort for day-to-day coding/agentic work, and users can cycle up
    /// when they want deeper reasoning. Every other model keeps the model's
    /// own API-side default (no forced effort).
    pub(super) fn default_reasoning_effort_for_model(model: &str) -> Option<String> {
        let key = jcode_provider_core::model_id::canonical(model);
        if key.starts_with("gpt-5.6-sol") {
            Some("low".to_string())
        } else {
            None
        }
    }

    pub(super) fn revalidate_reasoning_effort(&self) {
        let current = self
            .reasoning_effort
            .read()
            .map(|effort| effort.clone())
            .unwrap_or_else(|poisoned| poisoned.into_inner().clone());
        let Some(current) = current else {
            return;
        };
        if jcode_base::prompt::is_swarm_effort(&current)
            || self.available_efforts().contains(&current.as_str())
        {
            return;
        }
        jcode_base::logging::info(&format!(
            "Clearing OpenAI reasoning effort '{}' because model '{}' does not advertise it",
            current,
            self.model()
        ));
        match self.reasoning_effort.write() {
            Ok(mut effort) => *effort = None,
            Err(poisoned) => *poisoned.into_inner() = None,
        }
    }

    /// Resolve swarm effort only at the wire boundary, preserving the stored mode.
    pub(super) fn api_reasoning_effort(&self, effort: Option<&str>) -> Option<String> {
        self.api_reasoning_effort_with_swarm_root(
            effort,
            effort.and_then(jcode_base::prompt::swarm_root_reasoning_effort),
        )
    }

    pub(super) fn api_reasoning_effort_with_swarm_root(
        &self,
        effort: Option<&str>,
        swarm_root: Option<&str>,
    ) -> Option<String> {
        let effort = effort?;
        if !jcode_base::prompt::is_swarm_effort(effort) {
            return Some(effort.to_string());
        }
        let resolved = swarm_root.unwrap_or("max");
        let available = self.available_efforts();
        let ladder = jcode_provider_core::OPENAI_SELECTABLE_EFFORTS;
        let requested = ladder.iter().position(|e| *e == resolved)?;
        // Preserve the old strongest-advertised mapping for max. For lower
        // configured levels prefer the closest supported level at or below it,
        // falling back to the minimum on models with a restricted ladder.
        ladder[..=requested]
            .iter()
            .rev()
            .find(|candidate| available.contains(candidate))
            .or_else(|| {
                ladder.iter().find(|candidate| {
                    !jcode_base::prompt::is_swarm_effort(candidate) && available.contains(candidate)
                })
            })
            .map(|effort| (*effort).to_string())
    }
}
