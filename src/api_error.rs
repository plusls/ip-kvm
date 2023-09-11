use anyhow;
use axum::{http::StatusCode, Json};
use axum::response::{IntoResponse, Response};
use serde_json;

pub struct ApiError {
    status_code: StatusCode,
    error: anyhow::Error,
}

pub type Result<T> = std::result::Result<T, ApiError>;

// Tell axum how to convert `AppError` into a response.
impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        (
            self.status_code,
            Json(serde_json::json!({"error": format!("{}", self.error)}))
        )
            .into_response()
    }
}

// This enables using `?` on functions that return `Result<_, anyhow::Error>` to turn them into
// `Result<_, AppError>`. That way you don't need to do that manually.
impl<E> From<E> for ApiError
    where
        E: Into<anyhow::Error>,
{
    fn from(err: E) -> Self {
        Self { status_code: StatusCode::INTERNAL_SERVER_ERROR, error: err.into() }
    }
}