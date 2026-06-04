use crate::models::AppState;
use axum::{
    routing::{get, post},
    Router,
};
use std::{collections::HashMap, sync::Arc};
use tokio::sync::RwLock;

mod routes;

// mod routes;
mod models;
mod utils;

use routes::*;

#[tokio::main]
async fn main() {
    let state = Arc::new(AppState {
        sessions: RwLock::new(HashMap::new()),
    });

    let app = Router::new()
        .route("/", get(get_index))
        .route("/style.css", get(get_style))
        .route("/app.js", get(get_app))
        .route("/health", get(health))
        .route("/api/start", post(handle_start))
        .route("/api/answer", post(handle_answer))
        .route("/api/hint", post(handle_hint))
        .route("/api/next", post(handle_next))
        .with_state(state);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:8080").await.unwrap();
    println!("[+] Server running locally on http://localhost:8080");
    axum::serve(listener, app).await.unwrap();
}
