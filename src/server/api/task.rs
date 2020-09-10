use crate::messages::{TaskPriority, Token};
use crate::postoffice;
use crate::server::api::util::RequestExt;
use crate::server::api::State;
use crate::server::tokens::{increment_token, ProcessToken};
use chrono::{DateTime, Utc};
use hightide::Responder;
use tide::{Request, StatusCode};
use uuid::Uuid;

pub async fn create_token(req: Request<State>) -> tide::Result<impl Responder> {
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

    token_tx.send(ProcessToken(token, TaskPriority::High)).await;

    Ok(StatusCode::Created)
}
