use axum::{
    extract::FromRequestParts,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use axum_extra::headers::{Authorization, HeaderMapExt, authorization::Bearer};

use crate::server::api::{App, jwt};

pub mod global;
pub mod job;
pub mod project;

#[derive(sqlx::FromRow, serde::Serialize)]
pub struct StashName(String);

#[derive(sqlx::FromRow)]
pub struct StashData(Vec<u8>);

impl IntoResponse for StashData {
    fn into_response(self) -> Response {
        self.0.into_response()
    }
}

pub struct JwtSubject(String);

impl FromRequestParts<App> for JwtSubject {
    type Rejection = StatusCode;

    async fn from_request_parts(
        parts: &mut axum::http::request::Parts,
        app: &App,
    ) -> Result<Self, Self::Rejection> {
        let authz = parts
            .headers
            .typed_get::<Authorization<Bearer>>()
            .ok_or_else(|| StatusCode::UNAUTHORIZED)?;

        let keys = &app.jwt_keys;

        let subject = jwt::validate_stash_jwt(keys, authz.0.token()).map_err(|err| {
            tracing::warn!("error validating JWT: {}", err);
            StatusCode::UNAUTHORIZED
        })?;

        Ok(JwtSubject(subject))
    }
}
