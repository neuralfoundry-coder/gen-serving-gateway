//! Base64 encoding and decoding utilities

use base64::{engine::general_purpose::STANDARD, Engine};
use crate::error::{AppError, Result};

/// Encode binary data to base64 string
pub fn encode(data: &[u8]) -> String {
    STANDARD.encode(data)
}

/// Decode base64 string to binary data
pub fn decode(encoded: &str) -> Result<Vec<u8>> {
    // Handle data URL format (e.g., "data:image/png;base64,...")
    let data = if encoded.contains(",") {
        encoded.split(',').last().unwrap_or(encoded)
    } else {
        encoded
    };

    STANDARD
        .decode(data.trim())
        .map_err(|e| AppError::InvalidRequest(format!("Invalid base64 data: {}", e)))
}

/// Check if a string is valid base64
pub fn is_valid(data: &str) -> bool {
    let data = if data.contains(",") {
        data.split(',').last().unwrap_or(data)
    } else {
        data
    };
    
    STANDARD.decode(data.trim()).is_ok()
}

/// Get the image format from base64 data URL prefix
pub fn get_format_from_data_url(data_url: &str) -> Option<&str> {
    if data_url.starts_with("data:image/") {
        let end = data_url.find(';')?;
        Some(&data_url[11..end])
    } else {
        None
    }
}

/// Create a data URL from binary image data
pub fn create_data_url(data: &[u8], format: &str) -> String {
    let encoded = encode(data);
    format!("data:image/{};base64,{}", format, encoded)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encode_decode() {
        let original = b"Hello, World!";
        let encoded = encode(original);
        let decoded = decode(&encoded).unwrap();
        assert_eq!(original.as_slice(), decoded.as_slice());
    }

    #[test]
    fn test_data_url_decode() {
        let data_url = "data:image/png;base64,SGVsbG8sIFdvcmxkIQ==";
        let decoded = decode(data_url).unwrap();
        assert_eq!(b"Hello, World!", decoded.as_slice());
    }

    #[test]
    fn test_get_format() {
        assert_eq!(
            get_format_from_data_url("data:image/png;base64,abc"),
            Some("png")
        );
        assert_eq!(
            get_format_from_data_url("data:image/jpeg;base64,abc"),
            Some("jpeg")
        );
        assert_eq!(get_format_from_data_url("not a data url"), None);
    }
}

