use crate::dto::OptParams;
use image::ImageFormat;
use std::collections::HashMap;
use std::io::Cursor;
use crate::exception::AppError;

pub struct ImageService;

// Standard Cloudinary-style scales
pub const SCALE_THUMB: f32 = 0.15;
pub const SCALE_SM: f32 = 0.3;
pub const SCALE_MD: f32 = 0.5;
pub const SCALE_LG: f32 = 1.0;
pub const SCALE_RETINA: f32 = 2.0;

impl ImageService {
    /// Save triplets (webp, avif, jpg) during upload.
    pub async fn process_upload(content: &[u8]) -> Result<HashMap<String, Vec<u8>>, AppError> {
        let img = image::load_from_memory(content)
            .map_err(|e| anyhow::anyhow!(e))?;

        let mut results = HashMap::new();
        
        // Formats to generate
        let formats = vec![
            ("webp", ImageFormat::WebP),
            ("avif", ImageFormat::Avif),
            ("jpg", ImageFormat::Jpeg),
            ("png", ImageFormat::Png),
        ];

        for (ext, format) in formats {
            let mut buffer = Cursor::new(Vec::new());
            img.write_to(&mut buffer, format)
                .map_err(|e| anyhow::anyhow!(e))?;
            results.insert(ext.to_string(), buffer.into_inner());
        }

        Ok(results)
    }

    /// On-the-fly optimization (resize/convert).
    pub async fn optimize(
        content: &[u8],
        params: OptParams,
    ) -> Result<(Vec<u8>, String), AppError> {
        let mut img = image::load_from_memory(content)
            .map_err(|e| anyhow::anyhow!(e))?;

        // Handle Scale (numeric or named)
        if let Some(scale_val) = params.scale {
            let scale = scale_val.clamp(0.1, 2.0);
            let nwidth = (img.width() as f32 * scale) as u32;
            let nheight = (img.height() as f32 * scale) as u32;
            img = img.resize(nwidth, nheight, image::imageops::FilterType::Lanczos3);
        }

        // Convert type
        let format = match params.img_type.as_deref() {
            Some("webp") => ImageFormat::WebP,
            Some("avif") => ImageFormat::Avif,
            Some("jpeg") | Some("jpg") | Some("high-jpeg") => ImageFormat::Jpeg,
            Some("png") => ImageFormat::Png,
            _ => ImageFormat::WebP,
        };

        let mut buffer = Cursor::new(Vec::new());
        img.write_to(&mut buffer, format)
            .map_err(|e| anyhow::anyhow!(e))?;
        
        let content_type = match format {
            ImageFormat::WebP => "image/webp",
            ImageFormat::Avif => "image/avif",
            ImageFormat::Jpeg => "image/jpeg",
            ImageFormat::Png => "image/png",
            _ => "image/webp",
        };

        Ok((buffer.into_inner(), content_type.to_string()))
    }
}
