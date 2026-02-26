use dotenvy::dotenv;
use sea_orm::{Database, ConnectionTrait, Statement, DatabaseBackend};
use migration::{Migrator, MigratorTrait};
use std::net::SocketAddr;
use std::path::PathBuf;
use tokio::fs;
use image_service::{app, AppState};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenv().ok();
    tracing_subscriber::fmt::init();

    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    
    let (root_url, db_name) = split_db_url(&database_url);
    let root_db = Database::connect(&root_url).await?;
    
    let exists = root_db
        .query_one(Statement::from_string(
            DatabaseBackend::Postgres,
            format!("SELECT 1 FROM pg_database WHERE datname = '{}'", db_name),
        ))
        .await?
        .is_some();

    if !exists {
        tracing::info!("Database '{}' does not exist. Creating...", db_name);
        root_db.execute(Statement::from_string(
            DatabaseBackend::Postgres,
            format!("CREATE DATABASE \"{}\"", db_name),
        ))
        .await?;
    }

    let db = Database::connect(&database_url).await?;
    Migrator::up(&db, None).await?;

    // Use STORAGE_DIR from .env, defaulting to "storage"
    let storage_base = std::env::var("STORAGE_DIR").unwrap_or_else(|_| "storage".into());
    let upload_dir = PathBuf::from(storage_base).join("uploads");
    
    if !upload_dir.exists() {
        fs::create_dir_all(&upload_dir).await?;
    }

    let state = AppState {
        db,
        upload_dir,
    };

    let addr = SocketAddr::from(([0, 0, 0, 0], 3001));
    tracing::info!("listening on {}", addr);
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app(state)).await?;

    Ok(())
}

fn split_db_url(url: &str) -> (String, String) {
    let parts: Vec<&str> = url.split('/').collect();
    let db_name = parts.last().unwrap_or(&"dinerzflow_images").to_string();
    let root_url = parts[..parts.len()-1].join("/") + "/postgres";
    (root_url, db_name)
}
