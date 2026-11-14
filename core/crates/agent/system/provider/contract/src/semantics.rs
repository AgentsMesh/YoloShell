use serde::{Deserialize, Serialize};

/// Durable authority family used by Host/store/Worker composition.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AuthorityKind {
    None,
    GenericHookGeneration,
    ClaudeGeneration,
    CodexOccurrence,
}

/// Provider-owned Stop semantics. This is data, never a default method.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LifecycleSemantics {
    /// Stop carries no lifecycle fact (Claude, Qwen).
    StopIgnored,
    /// Stop is a candidate until a structured turn-ending record proves it (Codex).
    StopCandidateWithTurnProof,
    /// The pinned provider contract proves Stop is final.
    StopFinal,
}

/// How a provider's attention prompts can be answered.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InteractionSemantics {
    /// No attention interaction is declared.
    #[default]
    None,
    /// The native TUI owns the prompt; clients should open the terminal.
    Terminal,
    /// The provider bridge can deliver typed Allow/Deny (or equivalent)
    /// responses through the control channel.
    Typed,
}

/// Hub projection policy. Hook-authoritative providers report
/// `ObservationSource::ProviderHook` only when a verified authority binding is
/// available; process-only providers stay on `ProcessObserver`.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProjectionPolicy {
    pub hook_authoritative: bool,
}

impl ProjectionPolicy {
    pub const PROCESS_OBSERVER: Self = Self {
        hook_authoritative: false,
    };
    pub const HOOK_AUTHORITATIVE: Self = Self {
        hook_authoritative: true,
    };
}
