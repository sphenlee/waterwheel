use highnoon::{ws::WebSocket, Message};
use std::time::Duration;
use tracing::info;

pub async fn logs(mut ws: WebSocket) -> highnoon::Result<()> {
    loop {
        let msg = Message::text("Hello World!");
        info!("sending log line");
        match ws.send(msg).await {
            Ok(_) => info!("send OK"),
            Err(e) => {
                info!("send error: {}", e);
                break;
            }
        }
        tokio::time::sleep(Duration::from_secs(5)).await;
    }

    Ok(())
}
