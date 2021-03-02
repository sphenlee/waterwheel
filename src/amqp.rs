use crate::config;
use anyhow::Result;
use lapin::{Channel, Connection, ConnectionProperties};
use log::info;
use once_cell::sync::OnceCell;
use tokio_amqp::LapinTokioExt;

static AMQP_CONNECTION: OnceCell<Connection> = OnceCell::new();

pub async fn amqp_connect() -> Result<()> {
    info!("connecting to AMQP broker...");
    let addr: String = config::get_or("WATERWHEEL_AMQP_ADDR", "amqp://127.0.0.1:5672/%2f");

    let amqp_uri = addr.parse().map_err(anyhow::Error::msg)?;

    let conn =
        Connection::connect_uri(amqp_uri, ConnectionProperties::default().with_tokio()).await?;

    AMQP_CONNECTION
        .set(conn)
        .expect("AMQP connection is already set");
    info!("connected to AMQP broker");

    Ok(())
}

#[allow(unused)]
pub fn get_amqp_connection() -> &'static Connection {
    AMQP_CONNECTION.get().expect("AMQP connection not set")
}

pub async fn get_amqp_channel() -> Result<Channel> {
    let chan = AMQP_CONNECTION
        .get()
        .expect("AMQP connection not set")
        .create_channel()
        .await?;
    Ok(chan)
}
