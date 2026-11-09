use anyhow::{Result, anyhow, ensure};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use yolo_agent_adapter_contract::validate_launch_environment;
use yolo_agent_contract::{AgentBinding, AgentRecoveryAvailability, AgentState};
use yolo_agent_core::shell::{encode_command, login_arguments, user_shell};
use yolo_agent_providers::AgentRegistry;
use yolo_terminal_model::WorkerLaunchSpec;

const AUTOMATIC_RESUME_STATE_REJECTED: &str =
    "automatic provider resume requires a running or waiting agent binding";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ResumeIntent {
    Automatic,
    Explicit,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResumePolicy {
    Provider,
    ExplicitOnly,
    FailClosed,
}

impl ResumeIntent {
    pub fn policy(self, binding: &AgentBinding) -> ResumePolicy {
        if binding.recovery_availability != AgentRecoveryAvailability::Available {
            return ResumePolicy::FailClosed;
        }
        match (self, binding.state) {
            (Self::Explicit, _) => ResumePolicy::Provider,
            (
                Self::Automatic,
                AgentState::Active
                | AgentState::Running
                | AgentState::WaitingForInput
                | AgentState::WaitingForPermission,
            ) => ResumePolicy::Provider,
            (Self::Automatic, AgentState::Exited) => ResumePolicy::ExplicitOnly,
            (Self::Automatic, AgentState::Unknown) => ResumePolicy::Provider,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShellCommand {
    pub executable: PathBuf,
    pub args: Vec<String>,
    pub cwd: PathBuf,
}

pub fn shell_command(cwd: &Path) -> ShellCommand {
    ShellCommand {
        executable: user_shell(),
        args: login_arguments(),
        cwd: cwd.to_path_buf(),
    }
}

pub fn apply_resume_with_registry(
    spec: &mut WorkerLaunchSpec,
    binding: &AgentBinding,
    intent: ResumeIntent,
    registry: &AgentRegistry,
) -> Result<()> {
    ensure!(
        intent.policy(binding) == ResumePolicy::Provider,
        AUTOMATIC_RESUME_STATE_REJECTED
    );
    let adapter = registry
        .adapter(&binding.kind)
        .ok_or_else(|| anyhow!("unsupported agent kind: {:?}", binding.kind))?;
    let plan = adapter.resume_plan(binding);
    validate_launch_environment(&plan.environment)?;
    spec.initial_input = Some(encode_command(&plan));
    spec.environment = plan.environment;
    Ok(())
}

#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-unit-tests"))]
#[path = "../unit_tests/launch_tests.rs"]
mod tests;
