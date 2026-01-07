//! Response handling module - Base64, file storage, and URL generation

pub mod base64;
pub mod file;
pub mod url;

use crate::backend::traits::GeneratedImage;
use crate::error::Result;

/// Response format options
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResponseFormat {
    /// Base64 encoded JSON
    Base64Json,
    /// URL to the generated image
    Url,
    /// Direct file path (internal use)
    File,
}

impl ResponseFormat {
    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "b64_json" | "base64" => Self::Base64Json,
            "url" => Self::Url,
            "file" => Self::File,
            _ => Self::Url, // Default to URL
        }
    }
}

/// Response handler for processing generated images
pub struct ResponseHandler {
    file_handler: file::FileHandler,
    url_handler: url::UrlHandler,
}

impl ResponseHandler {
    /// Create a new response handler
    pub fn new(storage_path: String, url_prefix: String) -> Self {
        Self {
            file_handler: file::FileHandler::new(storage_path),
            url_handler: url::UrlHandler::new(url_prefix),
        }
    }

    /// Process a generated image based on the requested format
    pub async fn process(
        &self,
        image: GeneratedImage,
        format: ResponseFormat,
    ) -> Result<GeneratedImage> {
        match format {
            ResponseFormat::Base64Json => {
                // Image is already in base64 format
                Ok(image)
            }
            ResponseFormat::Url => {
                // If we have base64 data, save to file and return URL
                if let Some(b64_data) = &image.b64_json {
                    let file_path = self.file_handler.save_base64(b64_data).await?;
                    let url = self.url_handler.generate_url(&file_path);
                    
                    Ok(GeneratedImage {
                        b64_json: None,
                        url: Some(url),
                        revised_prompt: image.revised_prompt,
                        seed: image.seed,
                    })
                } else {
                    // Already has URL
                    Ok(image)
                }
            }
            ResponseFormat::File => {
                // Save to file and return file path
                if let Some(b64_data) = &image.b64_json {
                    let file_path = self.file_handler.save_base64(b64_data).await?;
                    
                    Ok(GeneratedImage {
                        b64_json: None,
                        url: Some(file_path),
                        revised_prompt: image.revised_prompt,
                        seed: image.seed,
                    })
                } else {
                    Ok(image)
                }
            }
        }
    }

    /// Process multiple images
    pub async fn process_batch(
        &self,
        images: Vec<GeneratedImage>,
        format: ResponseFormat,
    ) -> Result<Vec<GeneratedImage>> {
        let mut results = Vec::with_capacity(images.len());
        
        for image in images {
            results.push(self.process(image, format).await?);
        }
        
        Ok(results)
    }
}

