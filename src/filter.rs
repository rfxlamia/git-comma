//! Smart Diff Filter — excludes machine-generated files from AI diff input.

/// Domain type for filter options (Boolean Blindness prevention).
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum FilterMode {
    /// Default — filter machine-generated files
    Smart,
    /// User passed --no-filter — include everything
    NoFilter,
}

/// Why a file was excluded from the diff.
#[derive(Debug, Clone)]
pub enum ExclusionReason {
    BinaryFile,
    MachineGeneratedLockfile,
    MinifiedFile,
    HeuristicSize { added: u32, deleted: u32 },
}

/// A file that will be excluded from the diff.
#[derive(Debug, Clone)]
pub struct ExcludedFile {
    pub path: String,
    pub reason: ExclusionReason,
}

/// Result of filtering — carries excluded files AND whether all staged files were excluded.
#[derive(Debug)]
pub struct FilterResult {
    pub excluded: Vec<ExcludedFile>,
    pub all_excluded: bool,
}

/// Errors from filter operations.
#[derive(Debug, thiserror::Error)]
pub enum FilterError {
    #[error("Git numstat command failed")]
    NumstatFailed(#[from] std::io::Error),
    #[error("Failed to parse numstat line: '{line}'")]
    ParseError { line: String },
}

/// Extract basename from a path (last segment after /).
fn get_basename(path: &str) -> &str {
    path.rsplit('/').next().unwrap_or(path)
}

/// Returns true if the file is a known machine-generated lockfile (exact basename match only).
fn is_machine_generated_lockfile(basename: &str) -> bool {
    matches!(
        basename,
        "package-lock.json" | "pnpm-lock.yaml" | "yarn.lock"
            | "Cargo.lock" | "go.sum"
    )
}

/// Returns true if the file is a minified JavaScript or CSS file.
fn is_minified_file(basename: &str) -> bool {
    basename.ends_with(".min.js") || basename.ends_with(".min.css")
}

/// Converts excluded files to git :(exclude)path args.
/// Correct git pathspec syntax: :(exclude)path — parenthesis closes BEFORE the path.
pub fn build_git_exclude_args(excluded: &[ExcludedFile]) -> Vec<String> {
    excluded
        .iter()
        .map(|e| format!(":(exclude){}", e.path))
        .collect()
}

/// Runs the numstat heuristic pipeline and returns files to exclude.
pub fn filter_staged_files(
    mode: FilterMode,
) -> Result<FilterResult, FilterError> {
    // NoFilter mode: skip all checks, include everything
    if mode == FilterMode::NoFilter {
        return Ok(FilterResult {
            excluded: Vec::new(),
            all_excluded: false,
        });
    }

    // Run git diff --cached --numstat
    let output = std::process::Command::new("git")
        .args(["diff", "--cached", "--numstat"])
        .output()?;

    if !output.status.success() {
        return Err(FilterError::NumstatFailed(std::io::Error::new(
            std::io::ErrorKind::Other,
            String::from_utf8_lossy(&output.stderr),
        )));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut excluded = Vec::new();
    let mut total_staged = 0;

    for line in stdout.lines() {
        let parts: Vec<&str> = line.split('\t').collect();
        if parts.len() < 3 {
            // Malformed line — skip
            continue;
        }

        let path = parts[2];
        if path.is_empty() {
            // Empty path — skip
            continue;
        }

        total_staged += 1;
        let added_str = parts[0];
        let deleted_str = parts[1];

        // Binary file check
        if added_str == "-" && deleted_str == "-" {
            excluded.push(ExcludedFile {
                path: path.to_string(),
                reason: ExclusionReason::BinaryFile,
            });
            continue;
        }

        let added: u32 = added_str.parse().unwrap_or(0);
        let deleted: u32 = deleted_str.parse().unwrap_or(0);

        // Size heuristic check
        if added + deleted > 500 {
            excluded.push(ExcludedFile {
                path: path.to_string(),
                reason: ExclusionReason::HeuristicSize { added, deleted },
            });
            continue;
        }

        // Basename exact-match checks
        let basename = get_basename(path);

        if is_machine_generated_lockfile(basename) {
            excluded.push(ExcludedFile {
                path: path.to_string(),
                reason: ExclusionReason::MachineGeneratedLockfile,
            });
            continue;
        }

        if is_minified_file(basename) {
            excluded.push(ExcludedFile {
                path: path.to_string(),
                reason: ExclusionReason::MinifiedFile,
            });
            continue;
        }

        // Otherwise: SAFE — do not exclude
    }

    let all_excluded = excluded.len() == total_staged && total_staged > 0;

    Ok(FilterResult {
        excluded,
        all_excluded,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lockfile_exact_match() {
        assert!(is_machine_generated_lockfile("Cargo.lock"));
        assert!(is_machine_generated_lockfile("package-lock.json"));
        assert!(is_machine_generated_lockfile("pnpm-lock.yaml"));
        assert!(is_machine_generated_lockfile("yarn.lock"));
        assert!(is_machine_generated_lockfile("go.sum"));
    }

    #[test]
    fn test_lockfile_false_positives() {
        // These have "lock" or "sum" in the name but are NOT lockfiles
        assert!(!is_machine_generated_lockfile("lock-screen.jsx"));
        assert!(!is_machine_generated_lockfile("calculate_sum.ts"));
        assert!(!is_machine_generated_lockfile("user-lock.json"));
    }

    #[test]
    fn test_minified_file() {
        assert!(is_minified_file("bundle.min.js"));
        assert!(is_minified_file("styles.min.css"));
        assert!(!is_minified_file("main.js"));
        assert!(!is_minified_file("app.min.jsx")); // .jsx, not .js
        assert!(is_minified_file("normalize.min.css"));
    }

    #[test]
    fn test_basename_extraction() {
        assert_eq!(get_basename("src/main.rs"), "main.rs");
        assert_eq!(get_basename("frontend/src/components/lock-screen.jsx"), "lock-screen.jsx");
        assert_eq!(get_basename("Cargo.lock"), "Cargo.lock");
        assert_eq!(get_basename("package-lock.json"), "package-lock.json");
    }

    #[test]
    fn test_no_filter_returns_empty() {
        let result = filter_staged_files(FilterMode::NoFilter).unwrap();
        assert!(!result.all_excluded);
        assert!(result.excluded.is_empty());
    }

    #[test]
    fn test_build_git_exclude_args() {
        let excluded = vec![
            ExcludedFile {
                path: "Cargo.lock".into(),
                reason: ExclusionReason::MachineGeneratedLockfile,
            },
            ExcludedFile {
                path: "package-lock.json".into(),
                reason: ExclusionReason::MachineGeneratedLockfile,
            },
        ];
        let args = build_git_exclude_args(&excluded);
        assert_eq!(args, vec![":(exclude)Cargo.lock", ":(exclude)package-lock.json"]);
    }
}