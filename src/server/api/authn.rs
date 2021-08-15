use crate::server::api::request_ext::RequestExt;
use crate::server::api::State;
use highnoon::{Request, Responder, StatusCode};
use tracing::trace;

struct Login {
    username: String,
    password: String,
}

pub async fn login(mut req: Request<State>) -> highnoon::Result<impl Responder> {
    Ok("")
}