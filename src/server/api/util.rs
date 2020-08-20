use sqlx::PgPool;
use tide::{Request, Response, StatusCode, Body};
use super::State;
use serde::Serialize;

// extension methods for State
pub trait RequestExt {
    fn get_pool(&self) -> PgPool;
}

impl RequestExt for Request<State> {
    fn get_pool(&self) -> PgPool {
        self.state().pool.clone()
    }
}

// extension method to converting an Option into a Response
pub trait OptionExt<T: Serialize> {
    fn into_json_response(self) -> tide::Result<Response>;
}

impl<T: Serialize> OptionExt<T> for Option<T> {
    fn into_json_response(self) -> tide::Result<Response> {
        match self {
            None => Ok(Response::new(StatusCode::NotFound)),
            Some(t) => Ok(Response::builder(StatusCode::Ok)
                .body(Body::from_json(&t)?)
                .build()),
        }
    }
}
