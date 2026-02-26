pub mod models;
pub mod dto;
pub mod repositories;
pub mod services;
pub mod routes;
pub mod exception;

use axum::{
    routing::{get, post},
    Router,
};
use sea_orm::DatabaseConnection;
use std::path::PathBuf;

#[derive(Clone)]
pub struct AppState {
    pub db: DatabaseConnection,
    pub upload_dir: PathBuf,
}

pub fn app(state: AppState) -> Router {
    Router::new()
        .route("/presign", get(routes::generate_presign))
        .route("/uploads", post(routes::upload_image))
        .route("/:token", get(routes::serve_image))
        .route("/:token/opt", get(routes::serve_optimized_image))
        .with_state(state)
}

// Re-export for tests
pub use dto::{PresignResponse, OptParams};
