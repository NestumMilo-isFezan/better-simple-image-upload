use axum::http::StatusCode;
use axum_test::TestServer;
use axum_test::multipart::MultipartForm;
use image_service::{app, AppState, PresignResponse};
use sea_orm::Database;
use std::path::PathBuf;
use tokio::fs;
use dotenvy::dotenv;

#[tokio::test]
async fn test_full_image_lifecycle() {
    dotenv().ok();
    let database_url = std::env::var("DATABASE_URL").unwrap_or_else(|_| "postgres://user:password@localhost/dinerzflow_images".to_string());
    let secret = std::env::var("SECRET_KEY").unwrap_or_else(|_| "change-me".into());
    
    let db = match Database::connect(&database_url).await {
        Ok(d) => d,
        Err(_) => {
            println!("Skipping integration test: No database connection found.");
            return;
        }
    };

    // Organized test directory
    let upload_dir = PathBuf::from("storage/test-only");
    let state = AppState {
        db,
        upload_dir: upload_dir.clone(),
    };

    let server = TestServer::new(app(state)).unwrap();

    // 1. Get Presign Token
    let response = server.get("/presign")
        .add_header("X-Image-Service-Key", secret.clone())
        .await;
    response.assert_status_success();
    let presign: PresignResponse = response.json();

    // 2. Upload Image
    let image_bytes = fs::read("tests/sample/test_image.png").await.unwrap();
    let form = MultipartForm::new()
        .add_text("token", presign.token.to_string())
        .add_text("folder", "test_universal")
        .add_text("slug", "lifecycle-slug")
        .add_part("image", axum_test::multipart::Part::bytes(image_bytes).file_name("test.png"));

    let upload_resp = server.post("/uploads")
        .add_header("X-Image-Service-Key", secret)
        .multipart(form)
        .await;
    upload_resp.assert_status(StatusCode::CREATED);

    tokio::time::sleep(std::time::Duration::from_millis(500)).await;

    // 3. Serve Original (WebP or Fallback)
    let get_resp = server.get(&format!("/{}", presign.token)).await;
    get_resp.assert_status_success();

    // 4. Serve PNG format
    let png_resp = server.get(&format!("/{}/opt?type=png", presign.token)).await;
    png_resp.assert_status_success();
    assert_eq!(png_resp.header("content-type"), "image/png");

    // Cleanup specific test files
    let _ = fs::remove_dir_all("storage/test-only").await;
}
