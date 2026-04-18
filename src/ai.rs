// Re-export AI engine components from sibling modules
pub use crate::error::{AiError, CommitError};
pub use crate::prompt::build_payload;
pub use crate::sanitization::sanitize_response;
pub use crate::tui::{open_editor, prompt_action, prompt_custom_instruction, Action};

use crate::openrouter;
use std::io::{self, Write};
use std::path::Path;

/// Run the AI engine to generate a commit message from diff content.
pub fn run_ai_engine(api_key: &str, model: &str, diff_content: &str) -> Result<String, AiError> {
    // Build payload
    let payload = build_payload(model, diff_content, "");

    // Show loading indicator (no newline + flush before blocking call)
    print!("⏳ Menganalisis diff dan meracik pesan commit...");
    io::stdout().flush().ok();

    // Create client and call API
    let client = openrouter::Client::new(api_key.to_string());
    let raw_response = client
        .generate_commit_message(&payload)
        .map_err(|e| AiError::Api(format!("{}", e)))?;

    // Sanitize and return
    let sanitized = sanitize_response(&raw_response);
    if sanitized.is_empty() {
        return Err(AiError::EmptyResponse);
    }

    Ok(sanitized)
}

/// Regenerate commit message with custom instruction.
pub fn regenerate_with_instruction(
    api_key: &str,
    model: &str,
    diff_content: &str,
    instruction: &str,
) -> Result<String, AiError> {
    let payload = build_payload(model, diff_content, instruction);

    print!("⏳ Regenerating...");
    io::stdout().flush().ok();

    let client = openrouter::Client::new(api_key.to_string());
    let raw_response = client
        .generate_commit_message(&payload)
        .map_err(|e| AiError::Api(format!("{}", e)))?;

    let sanitized = sanitize_response(&raw_response);
    if sanitized.is_empty() {
        return Err(AiError::EmptyResponse);
    }

    Ok(sanitized)
}

/// Save draft to .git/comma_msg.txt for safety net.
pub fn save_draft(draft: &str, repo_root: &Path) -> Result<std::path::PathBuf, CommitError> {
    let backup_path = repo_root.join(".git").join("comma_msg.txt");
    std::fs::write(&backup_path, draft)?;
    Ok(backup_path)
}

/// Execute git commit with draft, with safety net.
pub fn commit_with_draft(draft: &str, repo_root: &Path) -> Result<(), CommitError> {
    // Save backup first (git commit fails first, backup survives)
    let backup_path = save_draft(draft, repo_root)?;

    // Execute git commit
    let commit_file = backup_path
        .to_str()
        .ok_or_else(|| CommitError::InvalidPath(backup_path.display().to_string()))?;

    let output = std::process::Command::new("git")
        .args(["commit", "-F", commit_file])
        .current_dir(repo_root)
        .output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(CommitError::HookFailed(stderr.to_string()));
    }

    // Remove backup on success
    std::fs::remove_file(&backup_path).ok();

    Ok(())
}

/// Get current git repository root.
pub fn get_repo_root() -> Option<std::path::PathBuf> {
    let output = std::process::Command::new("git")
        .args(["rev-parse", "--show-toplevel"])
        .output()
        .ok()?;
    if output.status.success() {
        Some(std::path::PathBuf::from(
            String::from_utf8_lossy(&output.stdout).trim(),
        ))
    } else {
        None
    }
}
