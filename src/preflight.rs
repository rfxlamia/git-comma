const SOFT_DIFF_LIMIT: usize = 15_000;

#[derive(Debug, thiserror::Error)]
pub enum PreflightError {
    #[error("Not a git repository")]
    NotGitRepo,
    #[error("Git command failed: {command}")]
    GitCommandFailed { command: String, source: std::io::Error },
    #[error("No staged files")]
    NoStagedFiles { unstaged: Vec<UnstagedFile> },
    #[error("Diff too large: {size} chars")]
    DiffTooLarge { size: usize },
}

#[derive(Debug, Clone)]
pub struct UnstagedFile {
    pub status: String,
    pub path: String,
}

#[derive(Debug)]
#[allow(dead_code)]
pub struct PreflightSuccess {
    pub diff_content: String,
}

fn is_git_repo() -> bool {
    std::process::Command::new("git")
        .args(["rev-parse", "--is-inside-work-tree"])
        .output()
        .map(|output| {
            let stdout = String::from_utf8_lossy(&output.stdout);
            stdout.trim() == "true"
        })
        .unwrap_or(false)
}

fn get_staged_files() -> Result<Vec<String>, std::io::Error> {
    let output = std::process::Command::new("git")
        .args(["diff", "--cached", "--name-only"])
        .output()?;
    Ok(String::from_utf8_lossy(&output.stdout)
        .lines()
        .map(String::from)
        .collect())
}

fn get_unstaged_files() -> Result<Vec<UnstagedFile>, std::io::Error> {
    let output = std::process::Command::new("git")
        .args(["status", "-s"])
        .output()?;
    Ok(String::from_utf8_lossy(&output.stdout)
        .lines()
        .filter_map(|line| {
            let status = line.chars().next()?;
            let path = line.strip_prefix(format!("{} ", status).as_str()).map(|p| p.trim())?;
            Some(UnstagedFile {
                status: status.to_string(),
                path: path.to_string(),
            })
        })
        .collect())
}

fn get_diff_content() -> Result<String, std::io::Error> {
    let output = std::process::Command::new("git")
        .args(["diff", "--cached"])
        .output()?;
    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

/// Runs pre-flight checks: git repo validity, staged files, diff size.
///
/// Returns `Ok(PreflightSuccess)` with diff content if all checks pass.
/// Returns `Err(PreflightError)` for any failure — does NOT print or exit.
pub fn run() -> Result<PreflightSuccess, PreflightError> {
    if !is_git_repo() {
        return Err(PreflightError::NotGitRepo);
    }

    let staged = get_staged_files().map_err(|e| PreflightError::GitCommandFailed {
        command: "git diff --cached --name-only".into(),
        source: e,
    })?;

    if staged.is_empty() {
        let unstaged = get_unstaged_files().map_err(|e| PreflightError::GitCommandFailed {
            command: "git status -s".into(),
            source: e,
        })?;
        return Err(PreflightError::NoStagedFiles { unstaged });
    }

    let diff_content = get_diff_content().map_err(|e| PreflightError::GitCommandFailed {
        command: "git diff --cached".into(),
        source: e,
    })?;

    if diff_content.len() > SOFT_DIFF_LIMIT {
        return Err(PreflightError::DiffTooLarge { size: diff_content.len() });
    }

    Ok(PreflightSuccess { diff_content })
}

/// Same as run() but skips the diff size check.
/// Used when user confirmed they want to proceed despite large diff.
pub fn run_with_diff_bypass() -> Result<PreflightSuccess, PreflightError> {
    if !is_git_repo() {
        return Err(PreflightError::NotGitRepo);
    }

    let staged = get_staged_files().map_err(|e| PreflightError::GitCommandFailed {
        command: "git diff --cached --name-only".into(),
        source: e,
    })?;

    if staged.is_empty() {
        let unstaged = get_unstaged_files().map_err(|e| PreflightError::GitCommandFailed {
            command: "git status -s".into(),
            source: e,
        })?;
        return Err(PreflightError::NoStagedFiles { unstaged });
    }

    let diff_content = get_diff_content().map_err(|e| PreflightError::GitCommandFailed {
        command: "git diff --cached".into(),
        source: e,
    })?;

    Ok(PreflightSuccess { diff_content })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_unstaged_file_parse_status_M() {
        let file = UnstagedFile {
            status: "M".to_string(),
            path: "src/main.rs".to_string(),
        };
        assert_eq!(file.status, "M");
        assert_eq!(file.path, "src/main.rs");
    }

    #[test]
    fn test_unstaged_file_parse_status_UU() {
        let file = UnstagedFile {
            status: "??".to_string(),
            path: ".env.example".to_string(),
        };
        assert_eq!(file.status, "??");
        assert_eq!(file.path, ".env.example");
    }

    #[test]
    fn test_preflight_error_display() {
        let err = PreflightError::NotGitRepo;
        assert_eq!(err.to_string(), "Not a git repository");
    }

    #[test]
    fn test_diff_too_large_error_display() {
        let err = PreflightError::DiffTooLarge { size: 23450 };
        assert_eq!(err.to_string(), "Diff too large: 23450 chars");
    }
}
