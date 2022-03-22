use super::State;
use crate::server::api::jwt;
use highnoon::{
    headers::{authorization::Bearer, Authorization},
    Error, Request, Responder, StatusCode,
};

pub mod global;
pub mod job;
pub mod project;

#[derive(sqlx::FromRow, serde::Serialize)]
struct StashName(String);

#[derive(sqlx::FromRow)]
struct StashData(Vec<u8>);

impl Responder for StashData {
    fn into_response(self) -> highnoon::Result<highnoon::Response> {
        self.0.into_response()
    }
}

pub fn get_jwt_subject(req: &Request<State>) -> highnoon::Result<String> {
    let jwt = req
        .header::<Authorization<Bearer>>()
        .ok_or_else(|| Error::http(StatusCode::UNAUTHORIZED))?;

    let keys = &req.state().server.jwt_keys;

    let subject = jwt::validate_stash_jwt(keys, jwt.0.token()).map_err(|err| {
        tracing::warn!("error validating JWT: {}", err);
        Error::http(StatusCode::UNAUTHORIZED)
    })?;

    Ok(subject)
}
