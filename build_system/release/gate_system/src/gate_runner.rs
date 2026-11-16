use crate::coverage_report::PinnedCoverageReport;
use crate::requirements::{GateRequirement, coverage_report_path};
use crate::snapshot::GateSnapshot;
use anyhow::{Context, Result, ensure};
use std::time::{SystemTime, UNIX_EPOCH};
use yolo_release_contracts::CLOCK_ERROR;
use yolo_release_contracts::{
    RUST_COVERAGE_INPUT_MISSING_ERROR, RUST_COVERAGE_OUTPUT_DUPLICATE_ERROR,
};
use yolo_release_evidence_model::{GateEvidence, GateToolEvidence};
use yolo_release_process_execution::ProcessCommand;
use yolo_release_tool_custody::TrustedBazel;

pub trait ProcessExecuting {
    fn execute(&self, command: &ProcessCommand) -> Result<()>;
    fn completed_at_unix_seconds(&self) -> Result<u64>;
}

pub struct SystemProcessExecutor;

impl ProcessExecuting for SystemProcessExecutor {
    fn execute(&self, specification: &ProcessCommand) -> Result<()> {
        let output = specification
            .execute()
            .with_context(|| command_context("execute", specification))?;
        ensure!(
            output.status.success(),
            "{}: {}",
            command_context("failed", specification),
            String::from_utf8_lossy(&output.stderr).trim()
        );
        Ok(())
    }

    fn completed_at_unix_seconds(&self) -> Result<u64> {
        Ok(SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .context(CLOCK_ERROR)?
            .as_secs())
    }
}

pub fn run_gate(
    requirement: GateRequirement,
    source_revision: &str,
    snapshot: &GateSnapshot,
    launcher: &TrustedBazel,
    executor: &dyn ProcessExecuting,
) -> Result<GateEvidence> {
    let execution = snapshot.execution()?;
    let mut commands = Vec::new();
    let mut coverage_report: Option<PinnedCoverageReport> = None;
    let gates = requirement
        .bazel_commands()
        .context("local gate runner requires a Bazel gate")?;
    for gate in gates {
        if gate.consumes_coverage_report() {
            let report = coverage_report
                .as_ref()
                .context(RUST_COVERAGE_INPUT_MISSING_ERROR)?;
            report.ensure_unchanged()?;
            ensure!(
                report.path() == coverage_report_path(&execution),
                RUST_COVERAGE_INPUT_MISSING_ERROR
            );
        }
        let specification = snapshot.command(launcher, *gate)?;
        snapshot.verify_integrity()?;
        launcher.ensure_unchanged()?;
        executor.execute(&specification)?;
        launcher.ensure_unchanged()?;
        snapshot.verify_integrity()?;
        if gate.produces_coverage_report() {
            ensure!(
                coverage_report.is_none(),
                RUST_COVERAGE_OUTPUT_DUPLICATE_ERROR
            );
            coverage_report = Some(PinnedCoverageReport::capture(snapshot)?);
        }
        if let Some(report) = &coverage_report {
            report.ensure_unchanged()?;
        }
        commands.push(specification);
    }
    let tool = GateToolEvidence::MacOsBazel {
        launcher: launcher.evidence(),
    };
    let evidence = commands
        .iter()
        .map(ProcessCommand::evidence)
        .collect::<Vec<_>>();
    ensure!(
        evidence == requirement.command_evidence(&tool, &execution)?,
        "release gate command evidence mismatch"
    );
    Ok(GateEvidence {
        gate: requirement.name.to_owned(),
        target: requirement.target.to_owned(),
        source_revision: source_revision.to_owned(),
        tool,
        execution,
        commands: evidence,
        completed_at_unix_seconds: executor.completed_at_unix_seconds()?,
    })
}

fn command_context(action: &str, specification: &ProcessCommand) -> String {
    format!(
        "{action} release command: {}",
        specification.evidence().join(" ")
    )
}
