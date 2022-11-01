use highnoon::{
    ws::{WebSocketReceiver, WebSocketSender},
    Message,
};
use std::time::Duration;
use tracing::info;

// TODO - this is a placeholder and was never implemented!
pub async fn _logs(mut _rx: WebSocketReceiver, mut tx: WebSocketSender) -> highnoon::Result<()> {
    loop {
        let msg = Message::text("Hello World!");
        info!("sending log line");
        match tx.send(msg).await {
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
