use anyhow::Result;
use axum::{
    routing::{get, post},
    Router,
};
use tokio::net::TcpListener;
use tracing::info;

mod controller;
mod entity;
mod error;
mod state;

use controller::{redirect, shorten};
use state::AppState;

const LISTEN_ADDR: &str = "127.0.0.1:9876";

pub struct App;

impl App {
    pub async fn run() -> Result<()> {
        let url = "postgres://postgres:01234567@localhost:54322/shortener";
        let state = AppState::try_new(url, LISTEN_ADDR).await?;
        info!("Connected to database: {url}");

        let listener = TcpListener::bind(LISTEN_ADDR).await?;
        info!("Listening on: {}", LISTEN_ADDR);

        let app = Router::new()
            .route("/", post(shorten))
            .route("/:id", get(redirect))
            .with_state(state);

        axum::serve(listener, app.into_make_service()).await?;
        Ok(())
    }
}
