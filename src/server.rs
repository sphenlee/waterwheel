use crate::spawn_and_log;
use crate::{amqp, db};
use anyhow::Result;

mod api;
mod execute;
mod progress;
pub mod tokens;
mod trigger_time;
mod triggers;

pub async fn run_server() -> Result<()> {
    db::create_pool().await?;
    amqp::amqp_connect().await?;

    let (execute_tx, execute_rx) = async_std::sync::channel(31); // TODO - tweak this?
    let (token_tx, token_rx) = async_std::sync::channel(31); // TODO - tweak this?

    spawn_and_log("triggers", triggers::process_triggers(token_tx.clone()));
    spawn_and_log("tokens", tokens::process_tokens(token_rx, execute_tx));
    spawn_and_log("executions", execute::process_executions(execute_rx));
    spawn_and_log("progress", progress::process_progress(token_tx));

    api::serve().await?;

    Ok(())
}
