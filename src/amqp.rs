use crate::config;
use anyhow::Result;
use lapin::{Channel, Connection, ConnectionProperties};
use once_cell::sync::OnceCell;
use tokio_amqp::LapinTokioExt;
use tracing::{info, warn};
use crate::config::Config;

pub async fn amqp_connect(config: &Config) -> Result<Connection> {
    info!("connecting to AMQP broker...");
    let addr = &config.amqp_addr;

    let amqp_uri = addr.parse().map_err(anyhow::Error::msg)?;

    let conn =
        Connection::connect_uri(amqp_uri, ConnectionProperties::default().with_tokio()).await?;

    info!("connected to AMQP broker");

    Ok(conn)
}
