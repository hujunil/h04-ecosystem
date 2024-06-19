use axum::http::StatusCode;
use serde::Serialize;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ShortenerError {
    #[error("Sqlx error: {0}")]
    SqlxError(#[from] sqlx::Error),
    #[error("Record not found")]
    NotFound,
    #[error("Server error")]
    InternalServerError,
    #[error("Id conflict")]
    IdConflict,
}

#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    error: String,
}

impl axum::response::IntoResponse for ShortenerError {
    fn into_response(self) -> axum::response::Response {
        let status_code = match self {
            ShortenerError::NotFound => StatusCode::NOT_FOUND,
            ShortenerError::InternalServerError => StatusCode::INTERNAL_SERVER_ERROR,
            ShortenerError::IdConflict => StatusCode::UNPROCESSABLE_ENTITY,
            ShortenerError::SqlxError(_) => StatusCode::INTERNAL_SERVER_ERROR,
        };

        (
            status_code,
            axum::Json(ErrorResponse {
                error: self.to_string(),
            }),
        )
            .into_response()
    }
}
