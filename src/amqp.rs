use crate::config::Config;
use anyhow::Result;
use lapin::{Connection, ConnectionProperties};
use tracing::info;

pub async fn amqp_connect(config: &Config) -> Result<Connection> {
    info!("connecting to AMQP broker...");
    let addr = &config.amqp_addr;

    let amqp_uri = addr.parse().map_err(anyhow::Error::msg)?;

    let conn =
        Connection::connect_uri(amqp_uri, ConnectionProperties::default()).await?;

    info!("connected to AMQP broker");

    Ok(conn)
}
