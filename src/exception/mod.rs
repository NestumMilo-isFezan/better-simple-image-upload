use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;
use sea_orm::DbErr;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum AppError {
    #[error("Database error: {0}")]
    Database(#[from] DbErr),

    #[error("Image processing error: {0}")]
    ImageProcessing(#[from] anyhow::Error),

    #[error("Invalid or expired token")]
    InvalidToken,

    #[error("Invalid API Key")]
    Unauthorized,

    #[error("File not found")]
    NotFound,

    #[error("Bad request: {0}")]
    BadRequest(String),
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, error_message): (StatusCode, String) = match self {
            AppError::Database(e) => {
                tracing::error!("Database error: {:?}", e);
                (StatusCode::INTERNAL_SERVER_ERROR, "Internal server error".to_string())
            }
            AppError::ImageProcessing(e) => {
                tracing::error!("Image processing error: {:?}", e);
                (StatusCode::INTERNAL_SERVER_ERROR, "Error processing image".to_string())
            }
            AppError::InvalidToken => (StatusCode::UNAUTHORIZED, "Invalid or expired token".to_string()),
            AppError::Unauthorized => (StatusCode::UNAUTHORIZED, "Invalid API Key".to_string()),
            AppError::NotFound => (StatusCode::NOT_FOUND, "Resource not found".to_string()),
            AppError::BadRequest(msg) => (StatusCode::BAD_REQUEST, msg),
        };

        let body = Json(json!({
            "error": error_message,
        }));

        (status, body).into_response()
    }
}
