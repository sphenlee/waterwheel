use crate::messages::{TaskPriority, Token};
use crate::postoffice;
use crate::server::api::request_ext::RequestExt;
use crate::server::api::State;
use crate::server::tokens::{increment_token, ProcessToken};
use chrono::{DateTime, Utc};
use highnoon::{Responder, Request, StatusCode};
use uuid::Uuid;

pub async fn create_token(req: Request<State>) -> highnoon::Result<impl Responder> {
    let token_tx = postoffice::post_mail::<ProcessToken>().await?;

    let task_id = req.param("id")?.parse::<Uuid>()?;
    let trigger_datetime = req.param("trigger_datetime")?.parse::<DateTime<Utc>>()?;

    let token = Token {
        task_id,
        trigger_datetime,
    };

    let pool = req.get_pool();
    let mut txn = pool.begin().await?;
    increment_token(&mut txn, &token).await?;
    txn.commit().await?;

    token_tx
        .send(ProcessToken::Increment(token, TaskPriority::High))?;

    Ok(StatusCode::CREATED)
}
