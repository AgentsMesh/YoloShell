use crate::constants::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct Prompt(String);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct InvalidPrompt;

impl std::fmt::Display for InvalidPrompt {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(INVALID_PROMPT)
    }
}
impl std::error::Error for InvalidPrompt {}

impl Prompt {
    pub fn new(text: String) -> Result<Self, InvalidPrompt> {
        text.try_into()
    }
    pub fn text(&self) -> &str {
        &self.0
    }
}

impl TryFrom<String> for Prompt {
    type Error = InvalidPrompt;
    fn try_from(text: String) -> Result<Self, Self::Error> {
        if text.trim().is_empty()
            || text.len() > MAX_PROMPT_BYTES
            || text.chars().any(|character| {
                character.is_control() && !matches!(character, LINE_FEED | HORIZONTAL_TAB)
            })
        {
            return Err(InvalidPrompt);
        }
        Ok(Self(text))
    }
}
impl From<Prompt> for String {
    fn from(prompt: Prompt) -> Self {
        prompt.0
    }
}
