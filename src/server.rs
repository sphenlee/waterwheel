use crate::postoffice;
use crate::spawn_and_log;
use crate::{amqp, db};
use anyhow::Result;

mod api;
mod execute;
mod heartbeat;
mod progress;
pub mod tokens;
mod trigger_time;
mod triggers;

pub async fn run_server() -> Result<()> {
    postoffice::open()?;

    db::create_pool().await?;
    amqp::amqp_connect().await?;

    spawn_and_log("triggers", triggers::process_triggers());
    spawn_and_log("tokens", tokens::process_tokens());
    spawn_and_log("executions", execute::process_executions());
    spawn_and_log("progress", progress::process_progress());
    spawn_and_log("heartbeat", heartbeat::process_heartbeats());

    api::serve().await?;

    Ok(())
}
