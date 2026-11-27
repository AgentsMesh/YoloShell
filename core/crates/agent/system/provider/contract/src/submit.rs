use std::time::Duration;

/// How a provider's prompt composer accepts the native submit payload.
///
/// The Worker owns the write mechanism; this is provider policy data. It is
/// not serialized and not a capability claim: a provider that declares a gap
/// still submits through the same authority, reservation and receipt fences.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum PromptSubmit {
    /// Bracketed paste, text, paste end and the submit key in one write.
    #[default]
    Atomic,
    /// Paste the text first, then send the submit key after `gap`. Composers
    /// may treat a submit key arriving in the same input burst as pasted
    /// content (Gemini CLI converts an Enter within 30 ms into a newline).
    DeferredEnter { gap: Duration },
}

impl PromptSubmit {
    /// The settle time between the pasted text and the submit key.
    pub fn enter_gap(self) -> Duration {
        match self {
            Self::Atomic => Duration::ZERO,
            Self::DeferredEnter { gap } => gap,
        }
    }
}

#[cfg(test)]
#[path = "../unit_tests/submit.rs"]
mod tests;
