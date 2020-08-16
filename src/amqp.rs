use anyhow::Result;
use lapin::{Connection, ConnectionProperties};
use async_amqp::LapinAsyncStdExt;
use async_std::sync::Arc;
use log::info;

pub async fn amqp_connection() -> Result<Arc<Connection>> {
    info!("connecting to AMQP broker...");
    let addr = std::env::var("WATERWHEEL_AMQP_ADDR").unwrap_or_else(|_| "amqp://127.0.0.1:5672/%2f".into());

    let conn = Connection::connect(
    &addr,
    ConnectionProperties::default().with_async_std()
    ).await?;

    info!("connected to AMQP broker");

    Ok(Arc::new(conn))
}
