use axum::response::{IntoResponse, Response};

#[derive(Debug)]
pub enum AppError {
    Internal(anyhow::Error),
    Http(Box<Response>),
}

impl AppError {
    pub fn http(r: impl IntoResponse) -> Self {
        Self::Http(Box::new(r.into_response()))
    }
}

impl<E> From<E> for AppError
where
    E: Into<anyhow::Error>,
{
    fn from(error: E) -> Self {
        AppError::Internal(error.into())
    }
}

impl axum::response::IntoResponse for AppError {
    fn into_response(self) -> Response {
        match self {
            AppError::Internal(error) => {
                tracing::error!("internal server error: {}", error);
                axum::http::StatusCode::INTERNAL_SERVER_ERROR.into_response()
            }
            AppError::Http(response) => *response,
        }
    }
}
