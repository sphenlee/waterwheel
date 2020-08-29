use tide::{Request, Response, StatusCode};
use crate::server::api::State;
use crate::postoffice;
use uuid::Uuid;
use chrono::{DateTime, Utc};
use crate::server::tokens::{Token, increment_token, ProcessToken};
use crate::server::api::util::RequestExt;

pub async fn create_token(req: Request<State>) -> tide::Result {
    let token_tx = postoffice::post_mail::<ProcessToken>().await?;

    let task_id = req.param::<Uuid>("id")?;
    let trigger_datetime = req.param::<DateTime<Utc>>("trigger_datetime")?;

    let token = Token {
        task_id,
        trigger_datetime,
    };

    let pool = req.get_pool();
    let mut txn = pool.begin().await?;
    increment_token(&mut txn, &token).await?;
    txn.commit().await?;

    token_tx.send(ProcessToken(token)).await;

    Ok(Response::builder(StatusCode::Created).build())
}
