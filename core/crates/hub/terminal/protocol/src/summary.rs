use serde::{Deserialize, Serialize};
use yolo_hub_identity::{AgentTarget, RuntimeTarget, TerminalAddress, WorkspaceAddress};

use crate::{
    MAX_DIRECTORY_BYTES, MAX_TERMINAL_NAME_BYTES, PublicError, SELECTOR_SEPARATOR,
    validation::metadata,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RuntimeStatus {
    Starting,
    Running,
    Finalizing,
    OutputBlocked,
    Exited,
    Lost,
    Unreachable,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RuntimeSummary {
    pub target: RuntimeTarget,
    pub status: RuntimeStatus,
    pub output_sequence: u64,
    pub size: Option<crate::TerminalSize>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub safe_termination: Option<bool>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TerminalRecoveryHint {
    Ready,
    Recovering,
    Resume,
    Recreate,
    Unavailable,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "TerminalSummaryFields", into = "TerminalSummaryFields")]
pub struct TerminalSummary(TerminalSummaryFields);

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TerminalSummaryFields {
    pub address: TerminalAddress,
    pub name: String,
    pub presentation: crate::TerminalPresentation,
    pub cwd: String,
    pub archived: bool,
    pub runtime: Option<RuntimeSummary>,
    pub agent: Option<AgentTarget>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub workspace: Option<WorkspaceAddress>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub recovery: Option<TerminalRecoveryHint>,
}

impl TerminalSummary {
    pub fn display_title(&self) -> &str {
        self.0
            .presentation
            .title
            .as_deref()
            .filter(|title| !title.is_empty())
            .unwrap_or(&self.0.name)
    }
    pub fn new(fields: TerminalSummaryFields) -> Result<Self, PublicError> {
        fields.try_into()
    }

    pub fn fields(&self) -> &TerminalSummaryFields {
        &self.0
    }

    /// Selectors accepted by the CLI and TUI: label, terminal id, full
    /// address, or the short `HUB/TERMINAL` form. Archived rows still match;
    /// callers exclude them where needed.
    pub fn matches_selector(&self, selector: &str) -> bool {
        let fields = self.fields();
        selector == fields.name
            || selector == fields.address.terminal_id.to_string()
            || selector == fields.address.to_string()
            || selector
                == format!(
                    "{}{SELECTOR_SEPARATOR}{}",
                    fields.address.hub_id, fields.address.terminal_id
                )
    }

    pub fn mark_unreachable(&mut self) {
        self.0.recovery = Some(TerminalRecoveryHint::Unavailable);
        if let Some(runtime) = self.0.runtime.as_mut() {
            runtime.safe_termination = None;
            if !matches!(runtime.status, RuntimeStatus::Exited | RuntimeStatus::Lost) {
                runtime.status = RuntimeStatus::Unreachable;
            }
        }
    }
}

impl TryFrom<TerminalSummaryFields> for TerminalSummary {
    type Error = PublicError;

    fn try_from(fields: TerminalSummaryFields) -> Result<Self, Self::Error> {
        if fields.presentation.validate().is_err() {
            return Err(PublicError::InvalidMetadata);
        }
        if !metadata(&fields.name, MAX_TERMINAL_NAME_BYTES)
            || !metadata(&fields.cwd, MAX_DIRECTORY_BYTES)
        {
            return Err(PublicError::InvalidMetadata);
        }
        if fields
            .runtime
            .is_some_and(|runtime| runtime.target.address != fields.address)
        {
            return Err(PublicError::TargetMismatch);
        }
        if fields
            .agent
            .is_some_and(|agent| agent.address.hub_id != fields.address.hub_id)
            || fields
                .workspace
                .is_some_and(|workspace| workspace.hub_id != fields.address.hub_id)
        {
            return Err(PublicError::OriginMismatch);
        }
        Ok(Self(fields))
    }
}

impl From<TerminalSummary> for TerminalSummaryFields {
    fn from(value: TerminalSummary) -> Self {
        value.0
    }
}
