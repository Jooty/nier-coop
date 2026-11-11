use std::fmt;

pub const MAX_NAME_BYTES: usize = 16;
const ALLOWED_PUNCTUATION: [char; 3] = [' ', '-', '_'];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NameError {
    Empty,
    TooLong,
    BadCharacter,
}

impl fmt::Display for NameError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Empty => write!(formatter, "Type a name first."),
            Self::TooLong => write!(
                formatter,
                "The name can be at most {MAX_NAME_BYTES} letters."
            ),
            Self::BadCharacter => write!(
                formatter,
                "The name can only use letters, digits, spaces, - and _."
            ),
        }
    }
}

pub fn validate_name(text: &str) -> Result<String, NameError> {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return Err(NameError::Empty);
    }
    if !trimmed.chars().all(|character| {
        character.is_ascii_alphanumeric() || ALLOWED_PUNCTUATION.contains(&character)
    }) {
        return Err(NameError::BadCharacter);
    }
    if trimmed.len() > MAX_NAME_BYTES {
        return Err(NameError::TooLong);
    }
    Ok(trimmed.to_owned())
}
