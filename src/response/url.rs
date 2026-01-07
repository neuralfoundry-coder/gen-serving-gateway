//! URL generation for stored images

use std::path::Path;

/// Handler for URL generation
pub struct UrlHandler {
    url_prefix: String,
}

impl UrlHandler {
    /// Create a new URL handler
    pub fn new(url_prefix: String) -> Self {
        // Ensure URL prefix doesn't end with slash
        let url_prefix = url_prefix.trim_end_matches('/').to_string();
        Self { url_prefix }
    }

    /// Generate a URL for a file path
    pub fn generate_url(&self, file_path: &str) -> String {
        // Extract filename from path
        let filename = Path::new(file_path)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or(file_path);

        format!("{}/{}", self.url_prefix, filename)
    }

    /// Generate a URL with additional path segments
    pub fn generate_url_with_path(&self, segments: &[&str]) -> String {
        let path = segments.join("/");
        format!("{}/{}", self.url_prefix, path)
    }

    /// Parse a URL to extract the filename
    pub fn extract_filename(&self, url: &str) -> Option<String> {
        url.strip_prefix(&format!("{}/", self.url_prefix))
            .or_else(|| url.rsplit('/').next())
            .map(String::from)
    }

    /// Check if a URL belongs to this handler
    pub fn is_local_url(&self, url: &str) -> bool {
        url.starts_with(&self.url_prefix)
    }

    /// Get the URL prefix
    pub fn prefix(&self) -> &str {
        &self.url_prefix
    }

    /// Update the URL prefix
    pub fn set_prefix(&mut self, prefix: String) {
        self.url_prefix = prefix.trim_end_matches('/').to_string();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_url() {
        let handler = UrlHandler::new("http://localhost:8080/images".to_string());
        
        assert_eq!(
            handler.generate_url("/path/to/image.png"),
            "http://localhost:8080/images/image.png"
        );
        
        assert_eq!(
            handler.generate_url("image.png"),
            "http://localhost:8080/images/image.png"
        );
    }

    #[test]
    fn test_extract_filename() {
        let handler = UrlHandler::new("http://localhost:8080/images".to_string());
        
        assert_eq!(
            handler.extract_filename("http://localhost:8080/images/image.png"),
            Some("image.png".to_string())
        );
    }

    #[test]
    fn test_is_local_url() {
        let handler = UrlHandler::new("http://localhost:8080/images".to_string());
        
        assert!(handler.is_local_url("http://localhost:8080/images/test.png"));
        assert!(!handler.is_local_url("http://example.com/test.png"));
    }
}

