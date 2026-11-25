use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::cmp::Ordering;
use std::hash::{Hash, Hasher};

use crate::constants::{CLAUDE_CODE_KIND, CODEX_KIND};

#[derive(Debug, Clone)]
pub enum AgentKind {
    Codex,
    ClaudeCode,
    Other(String),
}

impl AgentKind {
    pub fn from_name(name: impl Into<String>) -> Self {
        let name = name.into();
        match name.as_str() {
            CODEX_KIND => Self::Codex,
            CLAUDE_CODE_KIND => Self::ClaudeCode,
            _ => Self::Other(name),
        }
    }

    pub fn as_str(&self) -> &str {
        match self {
            Self::Codex => CODEX_KIND,
            Self::ClaudeCode => CLAUDE_CODE_KIND,
            Self::Other(name) => name,
        }
    }
}

impl PartialEq for AgentKind {
    fn eq(&self, other: &Self) -> bool {
        self.as_str() == other.as_str()
    }
}

impl Eq for AgentKind {}

impl PartialOrd for AgentKind {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for AgentKind {
    fn cmp(&self, other: &Self) -> Ordering {
        self.as_str().cmp(other.as_str())
    }
}

impl Hash for AgentKind {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.as_str().hash(state);
    }
}

impl Serialize for AgentKind {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(self.as_str())
    }
}

impl<'de> Deserialize<'de> for AgentKind {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        String::deserialize(deserializer).map(Self::from_name)
    }
}
