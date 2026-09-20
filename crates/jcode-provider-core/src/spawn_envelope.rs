//! Spawn-envelope enforcement shared by the [`Provider`] default and by
//! runtimes that implement metered reservation for billed routes.

use crate::{
    EventStream, Message, Provider, Result, SpawnExecutionEnvelope, ToolDefinition,
    with_spawn_deadline,
};

/// Spawn-envelope enforcement for a route that has no per-token spend to
/// reserve: refuse spend and router controls instead of ignoring them, and
/// enforce the spawn deadline.
///
/// Shared by [`Provider::complete_with_spawn_envelope`]'s default and by
/// runtimes that implement metered reservation for billed routes but must still
/// serve included-subscription spawns, which app-core admits without a budget.
pub async fn complete_with_spawn_envelope_without_budget<P>(
    provider: &P,
    messages: &[Message],
    tools: &[ToolDefinition],
    system: &str,
    resume_session_id: Option<&str>,
    envelope: &SpawnExecutionEnvelope,
) -> Result<EventStream>
where
    P: Provider + ?Sized,
{
    if envelope.max_micro_usd.is_some() {
        anyhow::bail!(
            "provider `{}` does not implement metered spawn reservation and settlement",
            provider.name()
        );
    }
    if envelope.router.is_some() {
        anyhow::bail!(
            "provider `{}` does not enforce spawn router policy",
            provider.name()
        );
    }
    let deadline = match envelope.deadline_secs {
        Some(0) => anyhow::bail!("spawn execution deadline_secs must be greater than zero"),
        Some(_) => envelope.deadline_at().map(tokio::time::Instant::from_std),
        None => None,
    };
    if deadline.is_some_and(|deadline| deadline <= tokio::time::Instant::now()) {
        anyhow::bail!("spawn execution deadline exceeded before provider call");
    }
    let completion = provider.complete(messages, tools, system, resume_session_id);
    let stream = if let Some(deadline) = deadline {
        tokio::time::timeout_at(deadline, completion)
            .await
            .map_err(|_| {
                anyhow::anyhow!("spawn execution deadline exceeded before stream opened")
            })??
    } else {
        completion.await?
    };
    Ok(match deadline {
        Some(deadline) => with_spawn_deadline(stream, deadline),
        None => stream,
    })
}
