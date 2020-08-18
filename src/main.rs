#![feature(never_type)]

use anyhow::Result;
use async_std::future::Future;
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

fn spawn_and_log(name: &str, future: impl Future<Output = Result<!>> + Send + 'static) {
    task::Builder::new()
        .name(name.to_owned())
        .spawn(future)
        .map_err(|err| {
            panic!("process '{}' failed: {}", name, err)
        }).unwrap();
}

async fn main_inner() -> Result<()> {
    db::create_pool().await?;
    amqp::amqp_connect().await?;

    let (execute_tx, execute_rx) = async_std::sync::channel(31); // TODO - tweak this?
    let (token_tx, token_rx) = async_std::sync::channel(31); // TODO - tweak this?

    spawn_and_log("triggers", triggers::process_triggers(token_tx));
    spawn_and_log(
        "tokens",
        tokens::process_tokens(token_rx, execute_tx.clone()),
    );
    spawn_and_log("executions", execute::process_executions(execute_rx));
    spawn_and_log("heartbeats", heartbeat::process_heartbeats(execute_tx));

    let mut app = tide::new();
    app.at("/")
        .get(|_req| async { Ok("Hello from Waterwheel!") });

    let host =
        std::env::var("WATERWHEEL_SERVER_ADDR").unwrap_or_else(|_| "127.0.0.1:8080".to_owned());

    app.listen(host).await?;

    Ok(())
}
