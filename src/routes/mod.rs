use axum::{
    extract::{Multipart, Path, Query, State},
    http::{header, HeaderMap, StatusCode},
    response::{Response, IntoResponse},
    Json,
};
use crate::AppState;
use crate::dto::{PresignResponse, OptParams};
use crate::repositories::{SignedUrlRepo, ImageRepo};
use crate::services::ImageService;
use crate::exception::AppError;
use tokio::fs;
use uuid::Uuid;
use image::ImageFormat;
use std::io::Cursor;

// Helper to sanitize paths
fn sanitize(s: &str) -> String {
    s.chars()
        .filter(|c| c.is_alphanumeric() || *c == '-' || *c == '_')
        .collect()
}

// Helper for security header
fn verify_key(headers: &HeaderMap) -> Result<(), AppError> {
    let secret = std::env::var("SECRET_KEY").unwrap_or_else(|_| "change-me".into());
    let key = headers.get("X-Image-Service-Key")
        .and_then(|h| h.to_str().ok())
        .ok_or(AppError::Unauthorized)?;

    if key != secret {
        return Err(AppError::Unauthorized);
    }
    Ok(())
}

pub async fn generate_presign(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<impl IntoResponse, AppError> {
    verify_key(&headers)?;
    let token = Uuid::new_v4();
    SignedUrlRepo::create(&state.db, token).await?;

    let base_url = std::env::var("BASE_URL").unwrap_or_else(|_| "http://localhost:3001".into());

    Ok(Json(PresignResponse {
        token,
        signed_url: format!("{}/{}", base_url, token),
    }))
}

pub async fn upload_image(
    State(state): State<AppState>,
    headers: HeaderMap,
    mut multipart: Multipart,
) -> Result<StatusCode, AppError> {
    verify_key(&headers)?;

    let mut token = None;
    let mut folder = String::from("general");
    let mut slug = String::from("default");
    let mut file_data = None;
    let mut filename = None;

    while let Some(field) = multipart.next_field().await.map_err(|e| AppError::BadRequest(e.to_string()))? {
        let name = field.name().unwrap_or_default().to_string();
        match name.as_str() {
            "token" => {
                let text = field.text().await.unwrap_or_default();
                token = Some(Uuid::parse_str(&text).map_err(|_| AppError::BadRequest("Invalid token format".into()))?);
            }
            "folder" => folder = sanitize(&field.text().await.unwrap_or_default()),
            "slug" => slug = sanitize(&field.text().await.unwrap_or_default()),
            "image" => {
                filename = field.file_name().map(|s| s.to_string());
                file_data = Some(field.bytes().await.map_err(|e| anyhow::anyhow!(e))?);
            }
            _ => {}
        }
    }

    let token = token.ok_or(AppError::BadRequest("Token is required".into()))?;
    let file_data = file_data.ok_or(AppError::BadRequest("Image is required".into()))?;

    let _ = SignedUrlRepo::find_by_token(&state.db, token).await?
        .ok_or(AppError::InvalidToken)?;

    let base_dir = format!("{}/{}", folder, slug);
    let full_base_path = state.upload_dir.join(&base_dir);
    fs::create_dir_all(&full_base_path).await.map_err(|e| anyhow::anyhow!(e))?;

    let original_ext = filename.as_ref()
        .and_then(|f| std::path::Path::new(f).extension())
        .and_then(|s| s.to_str())
        .unwrap_or("bin");
    
    let original_filename = format!("{}.original.{}", token, original_ext);
    let original_path = full_base_path.join(&original_filename);
    fs::write(&original_path, &file_data).await.map_err(|e| anyhow::anyhow!(e))?;

    let metadata = serde_json::json!({
        "original_filename": filename,
        "size": file_data.len(),
        "original_stored_as": original_filename,
    });

    ImageRepo::create(&state.db, token, base_dir.clone(), metadata, vec![]).await?;

    // --- PROCESS 1: WebP (High Priority) ---
    let state_webp = state.clone();
    let data_webp = file_data.clone();
    let path_webp = full_base_path.clone();
    tokio::spawn(async move {
        if let Ok(img) = image::load_from_memory(&data_webp) {
            let mut buffer = Cursor::new(Vec::new());
            if img.write_to(&mut buffer, ImageFormat::WebP).is_ok() {
                let file_path = path_webp.join(format!("{}.webp", token));
                let _ = fs::write(file_path, buffer.into_inner()).await;
                
                // Atomic DB update for webp
                let _ = ImageRepo::add_formats(&state_webp.db, token, vec!["webp".to_string()]).await;
            }
        }
    });

    // --- PROCESS 2: Others (AVIF, JPG, PNG) ---
    let state_others = state.clone();
    let data_others = file_data.clone();
    let path_others = full_base_path.clone();
    tokio::spawn(async move {
        if let Ok(img) = image::load_from_memory(&data_others) {
            let formats_to_gen = vec![
                ("avif", ImageFormat::Avif),
                ("jpg", ImageFormat::Jpeg),
                ("png", ImageFormat::Png),
            ];

            let mut finished_exts = Vec::new();
            for (ext, format) in formats_to_gen {
                let mut buffer = Cursor::new(Vec::new());
                if img.write_to(&mut buffer, format).is_ok() {
                    let file_path = path_others.join(format!("{}.{}", token, ext));
                    if fs::write(file_path, buffer.into_inner()).await.is_ok() {
                        finished_exts.push(ext.to_string());
                    }
                }
            }

            // Atomic DB update for others
            let _ = ImageRepo::add_formats(&state_others.db, token, finished_exts).await;
        }
    });

    Ok(StatusCode::CREATED)
}

pub async fn serve_image(
    State(state): State<AppState>,
    Path(token): Path<Uuid>,
) -> Result<Response, AppError> {
    let image_model = ImageRepo::find_by_token(&state.db, token).await?
        .ok_or(AppError::NotFound)?;

    let folder_path = state.upload_dir.join(&image_model.file_path);
    
    let webp_path = folder_path.join(format!("{}.webp", token));
    if webp_path.exists() {
        let content = fs::read(&webp_path).await.map_err(|e| anyhow::anyhow!(e))?;
        return Ok(Response::builder()
            .header(header::CONTENT_TYPE, "image/webp")
            .body(axum::body::Body::from(content))
            .unwrap());
    }

    let original_name = image_model.metadata
        .as_ref()
        .and_then(|m| m.get("original_stored_as"))
        .and_then(|v| v.as_str())
        .ok_or(AppError::NotFound)?;

    let original_path = folder_path.join(original_name);
    let content = fs::read(&original_path).await.map_err(|_| AppError::NotFound)?;
    let mime = mime_guess::from_path(&original_path).first_or_octet_stream();

    Ok(Response::builder()
        .header(header::CONTENT_TYPE, mime.to_string())
        .body(axum::body::Body::from(content))
        .unwrap())
}

pub async fn serve_optimized_image(
    State(state): State<AppState>,
    Path(token): Path<Uuid>,
    Query(params): Query<OptParams>,
) -> Result<Response, AppError> {
    let image_model = ImageRepo::find_by_token(&state.db, token).await?
        .ok_or(AppError::NotFound)?;

    let folder_path = state.upload_dir.join(&image_model.file_path);
    let requested_ext = params.img_type.clone().unwrap_or_else(|| "webp".to_string());
    let file_path = folder_path.join(format!("{}.{}", token, requested_ext));
    
    if !file_path.exists() {
        let original_name = image_model.metadata
            .as_ref()
            .and_then(|m| m.get("original_stored_as"))
            .and_then(|v| v.as_str())
            .ok_or(AppError::NotFound)?;

        let original_path = folder_path.join(original_name);
        let content = fs::read(&original_path).await.map_err(|_| AppError::NotFound)?;
        let (optimized_content, content_type) = ImageService::optimize(&content, params).await?;
        
        return Ok(Response::builder()
            .header(header::CONTENT_TYPE, content_type)
            .body(axum::body::Body::from(optimized_content))
            .unwrap());
    }

    let content = fs::read(&file_path).await.map_err(|e| anyhow::anyhow!(e))?;
    
    match ImageService::optimize(&content, params).await {
        Ok((optimized_content, content_type)) => {
            Ok(Response::builder()
                .header(header::CONTENT_TYPE, content_type)
                .body(axum::body::Body::from(optimized_content))
                .unwrap())
        },
        Err(e) => Err(e)
    }
}
