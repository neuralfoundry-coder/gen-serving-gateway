//! File storage handler for generated images

use std::path::PathBuf;
use tokio::fs;
use tracing::debug;
use uuid::Uuid;

use crate::error::{AppError, Result};
use crate::response::base64;

/// Handler for file storage operations
pub struct FileHandler {
    storage_path: PathBuf,
}

impl FileHandler {
    /// Create a new file handler
    pub fn new(storage_path: String) -> Self {
        Self {
            storage_path: PathBuf::from(storage_path),
        }
    }

    /// Ensure the storage directory exists
    pub async fn ensure_storage_dir(&self) -> Result<()> {
        if !self.storage_path.exists() {
            fs::create_dir_all(&self.storage_path)
                .await
                .map_err(|e| AppError::Io(e))?;
            debug!(path = ?self.storage_path, "Created storage directory");
        }
        Ok(())
    }

    /// Save base64 encoded image data to a file
    pub async fn save_base64(&self, b64_data: &str) -> Result<String> {
        self.ensure_storage_dir().await?;

        // Decode base64 data
        let image_data = base64::decode(b64_data)?;

        // Detect image format from data
        let format = detect_image_format(&image_data).unwrap_or("png");

        // Generate unique filename
        let filename = format!("{}.{}", Uuid::new_v4(), format);
        let file_path = self.storage_path.join(&filename);

        // Write file
        fs::write(&file_path, &image_data)
            .await
            .map_err(|e| AppError::Io(e))?;

        debug!(path = ?file_path, size = image_data.len(), "Saved image file");

        Ok(file_path.to_string_lossy().to_string())
    }

    /// Save raw image data to a file
    pub async fn save_raw(&self, data: &[u8], format: &str) -> Result<String> {
        self.ensure_storage_dir().await?;

        // Generate unique filename
        let filename = format!("{}.{}", Uuid::new_v4(), format);
        let file_path = self.storage_path.join(&filename);

        // Write file
        fs::write(&file_path, data)
            .await
            .map_err(|e| AppError::Io(e))?;

        debug!(path = ?file_path, size = data.len(), "Saved image file");

        Ok(file_path.to_string_lossy().to_string())
    }

    /// Read an image file
    pub async fn read(&self, filename: &str) -> Result<Vec<u8>> {
        let file_path = self.storage_path.join(filename);
        
        fs::read(&file_path)
            .await
            .map_err(|e| AppError::Io(e))
    }

    /// Delete an image file
    pub async fn delete(&self, filename: &str) -> Result<()> {
        let file_path = self.storage_path.join(filename);
        
        fs::remove_file(&file_path)
            .await
            .map_err(|e| AppError::Io(e))
    }

    /// List all files in storage
    pub async fn list(&self) -> Result<Vec<String>> {
        let mut files = Vec::new();
        
        let mut entries = fs::read_dir(&self.storage_path)
            .await
            .map_err(|e| AppError::Io(e))?;

        while let Some(entry) = entries.next_entry().await.map_err(|e| AppError::Io(e))? {
            if let Some(name) = entry.file_name().to_str() {
                files.push(name.to_string());
            }
        }

        Ok(files)
    }

    /// Get the full path for a filename
    pub fn get_path(&self, filename: &str) -> PathBuf {
        self.storage_path.join(filename)
    }

    /// Clean up old files (files older than max_age_secs)
    pub async fn cleanup(&self, max_age_secs: u64) -> Result<usize> {
        let mut deleted = 0;
        let now = std::time::SystemTime::now();
        let max_age = std::time::Duration::from_secs(max_age_secs);

        let mut entries = fs::read_dir(&self.storage_path)
            .await
            .map_err(|e| AppError::Io(e))?;

        while let Some(entry) = entries.next_entry().await.map_err(|e| AppError::Io(e))? {
            if let Ok(metadata) = entry.metadata().await {
                if let Ok(modified) = metadata.modified() {
                    if let Ok(age) = now.duration_since(modified) {
                        if age > max_age {
                            if fs::remove_file(entry.path()).await.is_ok() {
                                deleted += 1;
                                debug!(path = ?entry.path(), "Deleted old file");
                            }
                        }
                    }
                }
            }
        }

        Ok(deleted)
    }
}

/// Detect image format from binary data using magic bytes
fn detect_image_format(data: &[u8]) -> Option<&'static str> {
    if data.len() < 8 {
        return None;
    }

    // PNG: 89 50 4E 47 0D 0A 1A 0A
    if data.starts_with(&[0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A]) {
        return Some("png");
    }

    // JPEG: FF D8 FF
    if data.starts_with(&[0xFF, 0xD8, 0xFF]) {
        return Some("jpg");
    }

    // GIF: GIF87a or GIF89a
    if data.starts_with(b"GIF87a") || data.starts_with(b"GIF89a") {
        return Some("gif");
    }

    // WebP: RIFF....WEBP
    if data.len() >= 12 && data.starts_with(b"RIFF") && &data[8..12] == b"WEBP" {
        return Some("webp");
    }

    // BMP: BM
    if data.starts_with(b"BM") {
        return Some("bmp");
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_png() {
        let png_header = [0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A];
        assert_eq!(detect_image_format(&png_header), Some("png"));
    }

    #[test]
    fn test_detect_jpeg() {
        let jpeg_header = [0xFF, 0xD8, 0xFF, 0xE0, 0x00, 0x10, 0x4A, 0x46];
        assert_eq!(detect_image_format(&jpeg_header), Some("jpg"));
    }
}

