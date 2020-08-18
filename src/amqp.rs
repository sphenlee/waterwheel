use anyhow::Result;
use async_amqp::LapinAsyncStdExt;
use lapin::{Connection, ConnectionProperties, Channel};
use log::info;
use once_cell::sync::OnceCell;

static AMQP_CONNECTION: OnceCell<Connection> = OnceCell::new();

pub async fn amqp_connect() -> Result<()> {
    info!("connecting to AMQP broker...");
    let addr = std::env::var("WATERWHEEL_AMQP_ADDR")
        .unwrap_or_else(|_| "amqp://127.0.0.1:5672/%2f".into());

    let conn = Connection::connect(&addr, ConnectionProperties::default().with_async_std()).await?;

    AMQP_CONNECTION.set(conn).expect("AMQP connection is already set");
    info!("connected to AMQP broker");

    Ok(())
}

#[allow(unused)]
pub fn get_amqp_connection() -> &'static Connection {
     AMQP_CONNECTION.get().expect("AMQP connection not set")
}

pub async fn get_amqp_channel() -> Result<Channel> {
    let chan = AMQP_CONNECTION.get().expect("AMQP connection not set").create_channel().await?;
    Ok(chan)
}
