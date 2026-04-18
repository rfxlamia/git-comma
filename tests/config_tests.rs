use std::path::Path;
use std::io::Write;
use tempfile::TempDir;

#[test]
fn test_load_config_success() {
    let tmp = TempDir::new().unwrap();
    let config_path = tmp.path().join("comma.json");
    let content = r#"{"api_key": "sk-or-v1-test", "model_id": "test/model"}"#;
    std::fs::write(&config_path, content).unwrap();

    let result = comma_cli::config::Config::load_from_path(&config_path);
    assert!(result.is_ok());
    let cfg = result.unwrap();
    assert_eq!(cfg.api_key, "sk-or-v1-test");
    assert_eq!(cfg.model_id, "test/model");
}

#[test]
fn test_save_config_atomic() {
    let tmp = TempDir::new().unwrap();
    let config_path = tmp.path().join("comma.json");

    let cfg = comma_cli::config::Config {
        api_key: "sk-or-v1-test".into(),
        model_id: "test/model".into(),
    };

    cfg.save(&config_path).unwrap();

    let loaded = std::fs::read_to_string(&config_path).unwrap();
    assert!(loaded.contains("sk-or-v1-test"));
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let meta = std::fs::metadata(&config_path).unwrap();
        let perm = meta.permissions().mode();
        assert_eq!(perm & 0o777, 0o600);
    }
}
