//! Unit tests for response handlers

use generative_img_serving::response::{base64, ResponseFormat};

#[test]
fn test_base64_encode_decode() {
    let original = b"Hello, World!";
    let encoded = base64::encode(original);
    let decoded = base64::decode(&encoded).unwrap();
    
    assert_eq!(original.as_slice(), decoded.as_slice());
}

#[test]
fn test_base64_decode_data_url() {
    let data_url = "data:image/png;base64,SGVsbG8sIFdvcmxkIQ==";
    let decoded = base64::decode(data_url).unwrap();
    
    assert_eq!(b"Hello, World!", decoded.as_slice());
}

#[test]
fn test_base64_is_valid() {
    assert!(base64::is_valid("SGVsbG8sIFdvcmxkIQ=="));
    assert!(base64::is_valid("data:image/png;base64,SGVsbG8sIFdvcmxkIQ=="));
    assert!(!base64::is_valid("not valid base64!!!"));
}

#[test]
fn test_get_format_from_data_url() {
    assert_eq!(
        base64::get_format_from_data_url("data:image/png;base64,abc"),
        Some("png")
    );
    assert_eq!(
        base64::get_format_from_data_url("data:image/jpeg;base64,abc"),
        Some("jpeg")
    );
    assert_eq!(
        base64::get_format_from_data_url("data:image/webp;base64,abc"),
        Some("webp")
    );
    assert_eq!(
        base64::get_format_from_data_url("not a data url"),
        None
    );
}

#[test]
fn test_create_data_url() {
    let data = b"test data";
    let data_url = base64::create_data_url(data, "png");
    
    assert!(data_url.starts_with("data:image/png;base64,"));
    
    // Verify we can decode it back
    let decoded = base64::decode(&data_url).unwrap();
    assert_eq!(data.as_slice(), decoded.as_slice());
}

#[test]
fn test_response_format_from_str() {
    assert_eq!(ResponseFormat::from_str("b64_json"), ResponseFormat::Base64Json);
    assert_eq!(ResponseFormat::from_str("base64"), ResponseFormat::Base64Json);
    assert_eq!(ResponseFormat::from_str("url"), ResponseFormat::Url);
    assert_eq!(ResponseFormat::from_str("file"), ResponseFormat::File);
    assert_eq!(ResponseFormat::from_str("unknown"), ResponseFormat::Url); // Default
}

#[test]
fn test_response_format_case_insensitive() {
    assert_eq!(ResponseFormat::from_str("B64_JSON"), ResponseFormat::Base64Json);
    assert_eq!(ResponseFormat::from_str("URL"), ResponseFormat::Url);
    assert_eq!(ResponseFormat::from_str("File"), ResponseFormat::File);
}

