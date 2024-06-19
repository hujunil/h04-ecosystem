use anyhow::Result;
use axum::extract::Path;
use axum::http::header;
use axum::{extract::State, http::StatusCode, response::IntoResponse, Json};
use serde::{Deserialize, Serialize};

use super::error::ShortenerError;
use super::state::AppState;

#[derive(Debug, Deserialize)]
pub struct ShortenRequest {
    url: String,
}

#[derive(Debug, Serialize)]
pub struct ShortenResponse {
    url: String,
}

pub async fn shorten(
    State(state): State<AppState>,
    Json(payload): Json<ShortenRequest>,
) -> Result<impl IntoResponse, ShortenerError> {
    let id = state.shorten(&payload.url).await?;

    let body = Json(ShortenResponse {
        url: format!("http://{}/{}", state.server_url(), id),
    });

    Ok((StatusCode::CREATED, body))
}

pub async fn redirect(
    Path(id): Path<String>,
    State(state): State<AppState>,
) -> Result<impl IntoResponse, ShortenerError> {
    let url = state.find_url(&id).await?;

    let mut headers = header::HeaderMap::new();
    headers.insert(header::LOCATION, url.parse().unwrap());

    Ok((StatusCode::PERMANENT_REDIRECT, headers))
}
