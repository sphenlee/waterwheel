#![feature(never_type)]

use anyhow::Result;
use async_std::task;

//mod api;
//mod model;
mod amqp;
mod db;
mod execute;
mod heartbeat;
mod tokens;
mod trigger_time;
mod triggers;

fn main() -> Result<()> {
    dotenv::dotenv().ok();
    env_logger::builder().format_timestamp_millis().init();

    task::block_on(main_inner())
}

async fn main_inner() -> Result<()> {
    db::create_pool().await?;

    let (execute_tx, execute_rx) = async_std::sync::channel(5); // TODO - tweak this?
    let (token_tx, token_rx) = async_std::sync::channel(5); // TODO - tweak this?

    task::spawn(triggers::process_triggers(token_tx));
    task::spawn(tokens::process_tokens(token_rx, execute_tx.clone()));
    task::spawn(execute::process_executions(execute_rx));
    task::spawn(heartbeat::process_heartbeats(execute_tx));

    let mut app = tide::new();
    app.at("/")
        .get(|_req| async { Ok("Hello from Waterwheel!") });

    let host =
        std::env::var("WATERWHEEL_SERVER_ADDR").unwrap_or_else(|_| "127.0.0.1:8080".to_owned());

    app.listen(host).await?;

    Ok(())
}
