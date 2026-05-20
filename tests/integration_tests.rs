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

/// Integration test: when all staged files are excluded by HeuristicSize (>500 lines),
/// the preflight should fall back to the full diff (not return static message).
#[test]
fn test_preflight_heuristic_size_fallback_to_full_diff() {
    use git_comma::filter::FilterMode;
    use git_comma::preflight::run_with_filter;

    // Create a temporary git repo
    let tmp_dir = std::env::temp_dir().join(format!("comma_test_heuristic_{}", uuid_simple()));
    fs::create_dir_all(&tmp_dir).unwrap();

    // Initialize git repo
    std::process::Command::new("git")
        .args(["init"])
        .current_dir(&tmp_dir)
        .output()
        .expect("Failed to init git repo");

    // Configure git user for commits
    std::process::Command::new("git")
        .args(["config", "user.email", "test@test.com"])
        .current_dir(&tmp_dir)
        .output()
        .unwrap();
    std::process::Command::new("git")
        .args(["config", "user.name", "Test"])
        .current_dir(&tmp_dir)
        .output()
        .unwrap();

    // Create a file with >500 lines (HeuristicSize threshold)
    let mut content = String::new();
    for i in 0..600 {
        content.push_str(&format!("line {}: this is a test line for the heuristic size filter\n", i));
    }
    let file_path = tmp_dir.join("large_file.txt");
    fs::write(&file_path, &content).unwrap();

    // Stage the file
    std::process::Command::new("git")
        .args(["add", "large_file.txt"])
        .current_dir(&tmp_dir)
        .output()
        .expect("Failed to stage file");

    // Change to the temporary directory for the test
    let original_dir = std::env::current_dir().unwrap();
    std::env::set_current_dir(&tmp_dir).unwrap();

    // Run preflight with Smart filter (default) and a large enough limit
    let result = run_with_filter(FilterMode::Smart, 100_000);

    // Restore original directory
    std::env::set_current_dir(&original_dir).unwrap();

    // The result should be Ok with the full diff content (not the static message)
    // because HeuristicSize exclusion should fall back to full diff
    assert!(result.is_ok(), "Expected Ok, got Err: {:?}", result.err());
    let success = result.unwrap();
    assert!(
        !success.is_static_message,
        "Expected full diff (not static message) for HeuristicSize exclusion"
    );
    assert!(
        success.diff_content.contains("line 0:"),
        "Diff content should contain the actual file content"
    );
    assert!(
        success.diff_content.contains("line 599:"),
        "Diff content should contain the full file content"
    );

    // Cleanup
    fs::remove_dir_all(&tmp_dir).ok();
}

/// Generate a simple unique identifier (no external crate needed)
fn uuid_simple() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let t = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
    format!("{}{}", t.as_secs(), t.subsec_nanos())
}
