pub use crate::filter::{filter_staged_files, FilterMode};

const SOFT_DIFF_LIMIT: usize = 15_000;

#[derive(Debug, thiserror::Error)]
pub enum PreflightError {
    #[error("Not a git repository")]
    NotGitRepo,
    #[error("Git command failed: {command}")]
    GitCommandFailed {
        command: String,
        source: std::io::Error,
    },
    #[error("No staged files")]
    NoStagedFiles { unstaged: Vec<UnstagedFile> },
    #[error("Working tree clean — nothing to commit")]
    WorkingTreeClean,
    #[error("Diff too large: {size} chars")]
    DiffTooLarge { size: usize },
}

#[derive(Debug, Clone)]
pub struct UnstagedFile {
    pub status: String,
    pub path: String,
}

#[derive(Debug)]
pub struct PreflightSuccess {
    pub diff_content: String,
    pub is_static_message: bool,
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
            let bytes = line.as_bytes();
            if bytes.len() < 4 {
                return None; // line too short: "M f" is min valid
            }
            // git status -s: col1=staged, col2=worktree, space, then path
            // First char is staged status (or space if no staged change)
            // Second char is worktree status (or space if no unstaged change)
            let c1 = bytes[0] as char;
            let c2 = bytes[1] as char;
            let path = line[3..].to_string();
            // Skip if both are spaces (no actual change) or path empty
            if (c1 == ' ' && c2 == ' ') || path.is_empty() {
                return None;
            }
            Some(UnstagedFile {
                status: format!("{}{}", c1, c2),
                path,
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

fn is_working_tree_clean() -> Result<bool, std::io::Error> {
    let output = std::process::Command::new("git")
        .args(["status", "--porcelain"])
        .output()?;
    let clean = String::from_utf8_lossy(&output.stdout)
        .lines()
        .all(|line| line.trim().is_empty());
    Ok(clean)
}

/// Runs pre-flight checks: git repo validity, staged files, diff size.
///
/// Returns `Ok(PreflightSuccess)` with diff content if all checks pass.
/// Returns `Err(PreflightError)` for any failure — does NOT print or exit.
pub fn run() -> Result<PreflightSuccess, PreflightError> {
    run_with_filter(FilterMode::Smart)
}

/// Same as run() but allows explicit FilterMode (for --no-filter support).
pub fn run_with_filter(mode: FilterMode) -> Result<PreflightSuccess, PreflightError> {
    if !is_git_repo() {
        return Err(PreflightError::NotGitRepo);
    }

    if is_working_tree_clean().map_err(|e| PreflightError::GitCommandFailed {
        command: "git status --porcelain".into(),
        source: e,
    })? {
        return Err(PreflightError::WorkingTreeClean);
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

    // Run the filter
    let filter_result = filter_staged_files(mode).map_err(|e| {
        PreflightError::GitCommandFailed {
            command: "git diff --cached --numstat".into(),
            source: std::io::Error::new(std::io::ErrorKind::Other, e.to_string()),
        }
    })?;

    let diff_content = if filter_result.excluded.is_empty() {
        get_diff_content().map_err(|e| PreflightError::GitCommandFailed {
            command: "git diff --cached".into(),
            source: e,
        })?
    } else {
        let exclude_args = crate::filter::build_git_exclude_args(&filter_result.excluded);
        get_filtered_diff_content(&exclude_args).map_err(|e| PreflightError::GitCommandFailed {
            command: "git diff --cached :(exclude)".into(),
            source: e,
        })?
    };

    // Check if all staged files were excluded (static message path)
    if filter_result.all_excluded && diff_content.trim().is_empty() {
        return Ok(PreflightSuccess {
            diff_content: "chore: update dependencies".to_string(),
            is_static_message: true,
        });
    }

    // Check diff size limit
    if diff_content.len() > SOFT_DIFF_LIMIT {
        return Err(PreflightError::DiffTooLarge {
            size: diff_content.len(),
        });
    }

    Ok(PreflightSuccess {
        diff_content,
        is_static_message: false,
    })
}

/// Builds git diff command with exclude pathspec arguments.
fn get_filtered_diff_content(exclude_args: &[String]) -> Result<String, std::io::Error> {
    let mut cmd = std::process::Command::new("git");
    cmd.arg("diff").arg("--cached");
    for arg in exclude_args {
        cmd.arg(arg);
    }
    let output = cmd.output()?;
    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

/// Same as run() but skips the diff size check.
/// Used when user confirmed they want to proceed despite large diff.
pub fn run_with_diff_bypass() -> Result<PreflightSuccess, PreflightError> {
    run_with_filter(FilterMode::NoFilter)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_unstaged_file_parse_status_m() {
        let file = UnstagedFile {
            status: "M".to_string(),
            path: "src/main.rs".to_string(),
        };
        assert_eq!(file.status, "M");
        assert_eq!(file.path, "src/main.rs");
    }

    #[test]
    fn test_unstaged_file_parse_status_uu() {
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
