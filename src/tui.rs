use inquire::{Editor, Select, Text};

/// Display AI result and get user action.
pub fn prompt_action(_draft: &str) -> Result<Action, inquire::InquireError> {
    let options = vec![
        "✅ Accept & Commit",
        "✏️  Edit Manual",
        "🔄 Regenerate",
        "❌ Cancel",
    ];

    let selection = Select::new("Select action:", options)
        .with_starting_cursor(0)
        .prompt()?;

    match selection {
        "✅ Accept & Commit" => Ok(Action::Accept),
        "✏️  Edit Manual" => Ok(Action::Edit),
        "🔄 Regenerate" => Ok(Action::Regenerate),
        "❌ Cancel" => Ok(Action::Cancel),
        _ => unreachable!(),
    }
}

pub fn prompt_custom_instruction() -> Result<String, inquire::InquireError> {
    Text::new("Custom instruction (empty = regenerate):").prompt()
}

pub fn open_editor(draft: &str) -> Result<String, inquire::InquireError> {
    Editor::new("Edit message:")
        .with_file_extension(".txt")
        .with_predefined_text(draft)
        .prompt()
}

#[derive(Debug, Clone, Copy)]
pub enum Action {
    Accept,
    Edit,
    Regenerate,
    Cancel,
}
