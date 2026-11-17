use crate::requirements::{GateCommand, GateKind, GateRequirement, LinuxGateProfile};
use anyhow::{Result, bail};
use std::path::Path;
use yolo_release_contracts::LINUX_GATE_TARGET;
use yolo_release_evidence_model::{GateExecutionEvidence, GateToolEvidence};

impl GateRequirement {
    pub(crate) const fn bazel(
        name: &'static str,
        target: &'static str,
        commands: &'static [GateCommand],
    ) -> Self {
        Self {
            name,
            target,
            kind: GateKind::Bazel(commands),
        }
    }

    pub(crate) const fn linux(name: &'static str, profile: LinuxGateProfile) -> Self {
        Self {
            name,
            target: LINUX_GATE_TARGET,
            kind: GateKind::Linux(profile),
        }
    }

    pub fn bazel_commands(self) -> Option<&'static [GateCommand]> {
        match self.kind {
            GateKind::Bazel(commands) => Some(commands),
            GateKind::Linux(_) => None,
        }
    }

    pub fn command_evidence(
        self,
        tool: &GateToolEvidence,
        execution: &GateExecutionEvidence,
    ) -> Result<Vec<Vec<String>>> {
        match (self.kind, tool) {
            (GateKind::Bazel(commands), GateToolEvidence::MacOsBazel { launcher }) => Ok(commands
                .iter()
                .map(|command| {
                    std::iter::once(launcher.execution_path.clone())
                        .chain(command.arguments(execution))
                        .collect()
                })
                .collect()),
            (
                GateKind::Linux(profile),
                GateToolEvidence::LinuxDocker {
                    docker_execution_path,
                    bazel_path,
                    container_user,
                    ..
                },
            ) => Ok(vec![
                crate::linux_gate_command::evidence(
                    profile,
                    execution,
                    Path::new(docker_execution_path),
                    Path::new(bazel_path),
                    container_user,
                )?,
                crate::linux_cgroup_gate_command::evidence(
                    profile,
                    execution,
                    Path::new(docker_execution_path),
                    Path::new(bazel_path),
                    container_user,
                )?,
            ]),
            _ => bail!("release gate tool kind mismatch"),
        }
    }
}
