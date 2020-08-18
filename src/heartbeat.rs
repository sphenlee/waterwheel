//use crate::db;
use crate::tokens::Token;
use async_std::sync::Sender;
//use chrono::Utc;
//use std::collections::HashMap;

pub async fn process_heartbeats(_execute_tx: Sender<Token>) -> anyhow::Result<!> {
    /*let pool = db::get_pool();

    let last_checkin = HashMap::new();

    let now = Utc::now();
    sqlx::query_as(
        "SELECT
            task_id,
            trigger_datetime,
        FROM token
        WHERE state = 'active'"
    )*/

    futures::future::pending().await
}
