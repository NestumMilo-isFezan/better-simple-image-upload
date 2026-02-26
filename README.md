# Better Simple Image Upload (BSIU)

A high-performance, secure Rust microservice for simple image uploads and automated optimization. Designed to offload heavy binary processing from your main application (Rails, Node, etc.) to a specialized sidecar.

## Use Case
Perfect for modern web applications that need:
- **Instant Upload UX**: Return a "Success" response to the user in milliseconds while heavy work happens in the background.
- **Auto-Optimization**: Automatically generate WebP, AVIF, JPEG, and PNG formats for every upload.
- **Secure Storage**: Protect your file system with API key authentication and path sanitization.
- **Scalable Serving**: Serve resized or re-formatted images on-the-fly with Cloudinary-style parameters.

## Core Features
- **Async Architecture**: Built with Axum and Tokio for maximum concurrency.
- **Parallel Background Workers**: Splits image encoding into separate parallel tasks (WebP is prioritized for fastest availability).
- **Intelligent Fallback Logic**: Automatically serves the original file or performs on-the-fly optimization if background processing isn't finished yet.
- **Atomic Database**: Uses SeaORM (PostgreSQL) with atomic JSONB updates to track available formats.
- **Docker Ready**: Modern `compose.yaml` and optimized `Dockerfile` for easy deployment.

## Getting Started

### 1. Setup Environments
Copy the example environment file and update your configuration:
```bash
cp .env.example .env
```

### 2.1 Using Mise or RustUp
For local development with an existing PostgreSQL instance:
```bash
# Ensure Rust is installed
cargo run
```

### 2.2 Using Docker Compose (Recommended)
Start the entire stack (App + Database) with one command:
```bash
docker compose up -d
```
*Note: The DB is forwarded to port `5433` by default to avoid conflicts with local Postgres.*

### 2.3 Using Dockerfile (if using local database)
To run only the application in a container while using an external database:
```bash
docker build -t bsiu .
docker run -p 3001:3001 --env-file .env bsiu
```

## API Reference

### Generate Token
`GET /presign`
- **Header**: `X-Image-Service-Key: <your-secret>`
- **Response**: `{ "token": "uuid", "signed_url": "http://.../uuid" }`

### Upload Image
`POST /uploads`
- **Header**: `X-Image-Service-Key: <your-secret>`
- **Body (Multipart)**:
  - `token`: UUID from presign.
  - `folder`: e.g., "vouchers"
  - `slug`: e.g., "summer-promo-2026"
  - `image`: The binary file.
- **Response**: `201 Created` (Instant).

### Serve Image
`GET /:token`
- **Behavior**: Serves optimized WebP. Falls back to original if background task is pending.

### Optimized Serving
`GET /:token/opt?type=avif&scale=0.5`
- **Params**: `type` (webp|avif|jpeg|png), `scale` (0.1 to 2.0).
- **Behavior**: Serves from disk. Performs on-the-fly optimization if requested format is not ready.

## Roadmap
- [ ] **S3 Support**: Integration for AWS S3, Cloudflare R2, and self-hosted MinIO.
- [ ] **Admin Panel**: Web interface to browse, delete, and monitor image storage/stats.
- [ ] **Custom Presets**: Define named presets (e.g., `?preset=avatar`) in config.
- [ ] **Advanced Security**: Support for signed JWT tokens for image viewing.

## Testing
```bash
cargo test
```

## Contribution
Contributions are welcome! Please feel free to submit a Pull Request. For major changes, please open an issue first to discuss what you would like to change.
