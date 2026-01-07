//! API endpoint integration tests

use generative_img_serving::config::Settings;

#[tokio::test]
async fn test_settings_load_default() {
    let settings = Settings::default();
    assert_eq!(settings.server.host, "0.0.0.0");
    assert_eq!(settings.server.port, 8080);
}

#[tokio::test]
async fn test_settings_validation() {
    let mut settings = Settings::default();
    settings.server.port = 8080;
    assert!(settings.validate().is_ok());
}

#[tokio::test]
async fn test_settings_invalid_port() {
    let mut settings = Settings::default();
    settings.server.port = 0;
    assert!(settings.validate().is_err());
}

