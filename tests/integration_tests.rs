use std::fs;
use std::path::PathBuf;

fn home_config_path_fake() -> PathBuf {
    let tmp = std::env::temp_dir();
    tmp.join("comma_test.json")
}

#[test]
fn test_corrupted_json_triggers_setup() {
    let path = home_config_path_fake();
    fs::write(&path, "not valid json {{{").unwrap();

    let content = fs::read_to_string(&path).unwrap();
    assert!(!content.is_empty());

    let result = git_comma::config::Config::load_from_path(&path);
    assert!(result.is_err());
    assert!(matches!(
        result.unwrap_err(),
        git_comma::config::ConfigError::MalformedJson
    ));

    fs::remove_file(&path).ok();
}
