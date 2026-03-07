use crate::server::api::App;
use axum::{
    extract::{
        Path, State,
        ws::{Message, WebSocket, WebSocketUpgrade},
    },
    response::Response,
};
use redis::{
    AsyncCommands, FromRedisValue,
    streams::{StreamReadOptions, StreamReadReply},
};
use tracing::{debug, trace};
use uuid::Uuid;

#[axum::debug_handler]
pub async fn logs(
    State(app): State<App>,
    Path(task_run_id): Path<Uuid>,
    ws: WebSocketUpgrade,
) -> Response {
    ws.on_upgrade(move |mut socket: WebSocket| async move {
        let mut redis = match app.redis_client.get_multiplexed_tokio_connection().await {
            Ok(conn) => conn,
            Err(_e) => return,
        };

        let key = format!("waterwheel-logs.{task_run_id}");
        let mut id = "0-0".to_owned();
        let opts = StreamReadOptions::default().block(60000).count(10);

        debug!("reading logs from {}", key);
        loop {
            trace!("reading starting at id {}", id);
            let reply: StreamReadReply = match redis
                .xread_options(&[key.as_str()], &[id.as_str()], &opts)
                .await
            {
                Ok(r) => r,
                Err(_) => return,
            };

            if reply.keys.is_empty() {
                trace!("key expired while tailing logs");
                return;
            }

            if reply.keys[0].ids.is_empty() {
                trace!("got empty response, reading from '$'");
                id = "$".to_string();
                continue;
            }

            for entry in &reply.keys[0].ids {
                trace!("got entry with id {}", entry.id);
                let data: String = match String::from_redis_value(&entry.map["data"]) {
                    Ok(d) => d,
                    Err(_) => return,
                };

                if socket.send(Message::Text(data.into())).await.is_err() {
                    return;
                }
            }

            id = reply.keys[0].ids.last().unwrap().id.clone();
        }
    })
}
