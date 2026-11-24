#[cfg(test)]
use crate::developer_release_command::DeveloperReleaseCommandRunner;
use crate::developer_release_command::SystemDeveloperReleaseCommandRunner;
use anyhow::Result;
use std::path::Path;
use yolo_release_evidence_model::ReleaseProvenance;

#[path = "developer_release_flow.rs"]
mod developer_release_flow;

pub struct DeveloperReleaseRequest<'a> {
    pub candidate_directory: &'a Path,
    pub output_directory: &'a Path,
    pub notary_profile: &'a str,
    pub acceptance_receipt: Option<&'a Path>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DeveloperReleaseCheckpoint {
    PreparedBeforeRename,
}

pub trait DeveloperReleaseHooks {
    fn checkpoint(&self, _checkpoint: DeveloperReleaseCheckpoint) -> Result<()> {
        Ok(())
    }
}

struct ProductionHooks;

impl DeveloperReleaseHooks for ProductionHooks {}

pub fn distribute(request: DeveloperReleaseRequest<'_>) -> Result<ReleaseProvenance> {
    let now = yolo_release_acceptance_receipt::current_unix_seconds()?;
    developer_release_flow::run(
        request,
        &SystemDeveloperReleaseCommandRunner,
        &ProductionHooks,
        now,
    )
}

#[cfg(test)]
pub fn distribute_public_with(
    request: DeveloperReleaseRequest<'_>,
    runner: &dyn DeveloperReleaseCommandRunner,
) -> Result<ReleaseProvenance> {
    let now = yolo_release_acceptance_receipt::current_unix_seconds()?;
    distribute_with_test_acceptance_key(request, runner, &ProductionHooks, now)
}

#[cfg(test)]
pub fn distribute_public_with_at(
    request: DeveloperReleaseRequest<'_>,
    runner: &dyn DeveloperReleaseCommandRunner,
    now_unix_seconds: u64,
) -> Result<ReleaseProvenance> {
    distribute_with_test_acceptance_key(request, runner, &ProductionHooks, now_unix_seconds)
}

#[cfg(test)]
pub fn distribute_public_with_hooks_at(
    request: DeveloperReleaseRequest<'_>,
    runner: &dyn DeveloperReleaseCommandRunner,
    hooks: &dyn DeveloperReleaseHooks,
    now_unix_seconds: u64,
) -> Result<ReleaseProvenance> {
    distribute_with_test_acceptance_key(request, runner, hooks, now_unix_seconds)
}

#[cfg(test)]
fn distribute_with_test_acceptance_key(
    request: DeveloperReleaseRequest<'_>,
    runner: &dyn DeveloperReleaseCommandRunner,
    hooks: &dyn DeveloperReleaseHooks,
    now_unix_seconds: u64,
) -> Result<ReleaseProvenance> {
    developer_release_flow::run_with_test_acceptance_key(
        request,
        runner,
        hooks,
        now_unix_seconds,
        yolo_release_acceptance_receipt::test_public_key(),
    )
}

#[cfg(test)]
pub fn distribute_with(
    request: DeveloperReleaseRequest<'_>,
    runner: &dyn DeveloperReleaseCommandRunner,
) -> Result<ReleaseProvenance> {
    developer_release_flow::run_without_acceptance(request, runner, &ProductionHooks)
}

#[cfg(test)]
pub fn distribute_with_hooks(
    request: DeveloperReleaseRequest<'_>,
    runner: &dyn DeveloperReleaseCommandRunner,
    hooks: &dyn DeveloperReleaseHooks,
) -> Result<ReleaseProvenance> {
    developer_release_flow::run_without_acceptance(request, runner, hooks)
}
