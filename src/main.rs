use anyhow::Result;
use async_std::task;
use futures::TryFutureExt;
use log::info;

//mod api;
//mod model;
mod amqp;
mod db;
mod execute;
mod trigger_time;
mod triggers;

fn main() -> Result<()> {
    dotenv::dotenv().ok();
    env_logger::builder().format_timestamp_millis().init();

    task::block_on(main_inner())
}

async fn main_inner() -> Result<()> {
    let pool = db::create_pool().await?;

    let (execute_tx, execute_rx) = async_std::sync::channel(5); // TODO - tweak this?

    let trigger_future = task::spawn(triggers::process_triggers(pool.clone(), execute_tx));
    let execute_future = task::spawn(execute::process_executions(pool.clone(), execute_rx));

    let mut app = tide::new();
    app.at("/")
        .get(|_req| async { Ok("Hello from Waterwheel!") });

    let host =
        std::env::var("WATERWHEEL_SERVER_ADDR").unwrap_or_else(|_| "127.0.0.1:8080".to_owned());
    let listener = app.listen(host).map_err(|ioe| ioe.into());

    futures::future::try_join3(trigger_future, execute_future, listener).await?;

    Ok(())
}
