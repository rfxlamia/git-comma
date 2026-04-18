/// Sanitize AI response: trim whitespace, remove potential markdown wrappers.
pub fn sanitize_response(raw: &str) -> String {
    let trimmed = raw.trim();

    // Remove common markdown wrappers
    let without_backticks = trimmed
        .strip_prefix("```\n")
        .and_then(|s| s.strip_suffix("```"))
        .or_else(|| trimmed.strip_prefix("```"))
        .unwrap_or(trimmed);

    // Remove common prefixes AI might add
    let without_prefix = without_backticks
        .strip_prefix("Here is the commit message:\n")
        .or_else(|| without_backticks.strip_prefix("Commit message:\n"))
        .or_else(|| without_backticks.strip_prefix("Subject: "))
        .unwrap_or(without_backticks);

    without_prefix.trim().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sanitize_trims_whitespace() {
        let input = "  \nfeat: add feature\n  ";
        assert_eq!(sanitize_response(input), "feat: add feature");
    }

    #[test]
    fn test_sanitize_removes_backticks() {
        let input = "```\nfeat: add feature\n```";
        assert_eq!(sanitize_response(input), "feat: add feature");
    }

    #[test]
    fn test_sanitize_removes_prefix() {
        let input = "Here is the commit message:\nfeat: add feature";
        assert_eq!(sanitize_response(input), "feat: add feature");
    }

    #[test]
    fn test_sanitize_preserves_multiline() {
        let input = "feat(ux): implement interactive checks\n\n- Add fallback for unstaged files\n- Keep terminal clean";
        assert_eq!(sanitize_response(input), input);
    }
}
