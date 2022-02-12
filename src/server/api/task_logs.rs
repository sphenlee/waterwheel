use highnoon::ws::{WebSocketReceiver, WebSocketSender};
use highnoon::Message;
use std::time::Duration;
use tracing::info;

pub async fn logs(mut tx: WebSocketSender, _rx: WebSocketReceiver) -> highnoon::Result<()> {
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
