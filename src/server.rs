use crate::util::spawn_retry;
use anyhow::Result;

mod api;
mod execute;
mod progress;
pub mod stash;
pub mod tokens;
mod trigger_time;
pub mod triggers;
mod updates;

pub async fn run_scheduler() -> Result<()> {
    spawn_retry("triggers", triggers::process_triggers);
    spawn_retry("tokens", tokens::process_tokens);
    spawn_retry("executions", execute::process_executions);
    spawn_retry("progress", progress::process_progress);
    spawn_retry("updates", updates::process_updates);

    let () = futures::future::pending().await; // wait forever

    Ok(())
}

pub async fn run_api() -> Result<()> {
    stash::load_keys()?;

    api::serve().await?;

    Ok(())
}
