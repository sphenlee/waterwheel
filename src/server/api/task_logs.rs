use super::State;
use highnoon::{
    ws::{WebSocketReceiver, WebSocketSender},
    Message, Request,
};
use redis::{
    streams::{StreamReadOptions, StreamReadReply},
    AsyncCommands,
};
use tracing::{debug, trace};

fn get_as_string(value: &redis::Value) -> highnoon::Result<String> {
    match value {
        redis::Value::Data(raw) => Ok(String::from_utf8(raw.clone())?),
        _ => Err(anyhow::anyhow!("data was not binary").into()),
    }
}

pub async fn logs(
    req: Request<State>,
    mut tx: WebSocketSender,
    mut _rx: WebSocketReceiver,
) -> highnoon::Result<()> {
    let mut redis = req.state().redis_client.get_tokio_connection().await?;

    let task_run_id = req.param("id")?;
    let key = format!("waterwheel-logs.{task_run_id}");
    let mut id = "0-0".to_owned();
    let opts = StreamReadOptions::default().block(60000).count(10);

    debug!("reading logs from {}", key);
    loop {
        trace!("reading starting at id {}", id);
        let reply: StreamReadReply = redis
            .xread_options(&[key.as_str()], &[id.as_str()], &opts)
            .await?;

        if reply.keys.is_empty() {
            trace!("key expired while tailing logs");
            return Ok(());
        }

        if reply.keys[0].ids.is_empty() {
            trace!("got empty response, reading from '$'");
            id = "$".to_string();
            continue;
        }

        for entry in &reply.keys[0].ids {
            trace!("got entry with id {}", entry.id);
            let data = get_as_string(&entry.map["data"])?;

            let msg = Message::text(data);
            tx.send(msg).await?;
        }

        id = reply.keys[0].ids.last().unwrap().id.clone();
    }
}
