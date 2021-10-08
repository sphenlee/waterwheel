use crate::config;
use crate::util::spawn_or_crash;
use anyhow::Result;
use tracing::warn;

mod api;
mod execute;
pub mod jwt;
mod progress;
pub mod tokens;
mod trigger_time;
pub mod triggers;
mod updates;

pub async fn run_scheduler() -> Result<()> {
    spawn_or_crash("triggers", triggers::process_triggers);
    spawn_or_crash("tokens", tokens::process_tokens);
    spawn_or_crash("executions", execute::process_executions);
    spawn_or_crash("progress", progress::process_progress);
    spawn_or_crash("updates", updates::process_updates);

    Ok(())
}

pub async fn run_api() -> Result<()> {
    if config::get().no_authz {
        warn!("authorization is disabled, this is not recommended in production");
    }

    jwt::load_keys()?;

    api::serve().await?;

    Ok(())
}
