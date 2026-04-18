/// System prompt for commit message generation.
/// Optimized for attention mechanics: role/task at START, constraint at END.
pub const SYSTEM_PROMPT: &str = r#"You are an expert software engineer. Write git commit messages from diffs.

Conventional Commits: feat, fix, refactor, chore, docs, style, test, perf, ci, build, revert

RULES:
1. Subject line ≤ 72 chars, imperative mood ("Add feature" not "Added feature")
2. Small/simple diff → subject line only
3. Complex diff → subject line, blank line, bullet points for WHAT and WHY
4. Raw text only. No markdown (```), no prefixes like "Subject:", no greetings.
5. No conversational text. Output begins immediately with the commit message.
6. Keep subject line short (under 10 words / 72 chars)."#;

/// Build API payload for chat completions.
pub fn build_payload(
    model: &str,
    diff_content: &str,
    custom_instruction: &str,
) -> serde_json::Value {
    let mut user_content = format!("Here is the git diff:\n{}", diff_content);

    if !custom_instruction.trim().is_empty() {
        user_content.push_str(&format!(
            "\n\n<custom_instruction>\n{}\n</custom_instruction>",
            custom_instruction.trim()
        ));
    }

    serde_json::json!({
        "model": model,
        "messages": [
            { "role": "system", "content": SYSTEM_PROMPT },
            { "role": "user", "content": user_content }
        ],
        "temperature": 0.2
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_payload_without_custom_instruction() {
        let payload = build_payload("mistral-nemo", "diff content here", "");
        let user_content = payload["messages"][1]["content"]
            .as_str()
            .expect("user content should be string");
        assert!(!user_content.contains("<custom_instruction>"));
        assert!(user_content.contains("diff content here"));
    }

    #[test]
    fn test_build_payload_with_custom_instruction() {
        let payload = build_payload("mistral-nemo", "diff content here", "Make it shorter");
        let user_content = payload["messages"][1]["content"]
            .as_str()
            .expect("user content should be string");
        assert!(user_content.contains("<custom_instruction>"));
        assert!(user_content.contains("Make it shorter"));
    }

    #[test]
    fn test_build_payload_model_and_temperature() {
        let payload = build_payload("claude-sonnet", "diff", "");
        assert_eq!(payload["model"].as_str().unwrap(), "claude-sonnet");
        assert_eq!(payload["temperature"].as_f64().unwrap(), 0.2);
    }
}
