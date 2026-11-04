use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Receipt {
    Unknown,
    Delivered,
    NotDelivered,
}

/// Definitive evidence from the effect owner, never inferred from an I/O error.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Completion {
    Delivered,
    NotDelivered,
}

impl From<Completion> for Receipt {
    fn from(outcome: Completion) -> Self {
        match outcome {
            Completion::Delivered => Self::Delivered,
            Completion::NotDelivered => Self::NotDelivered,
        }
    }
}
