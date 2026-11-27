use crate::requirements::{CoverageAction, GateCommand};
use std::path::{Path, PathBuf};
use yolo_release_contracts::*;
use yolo_release_evidence_model::GateExecutionEvidence;

impl GateCommand {
    pub(crate) const fn fixed(arguments: &'static [&'static str]) -> Self {
        Self {
            arguments,
            coverage_action: CoverageAction::None,
        }
    }

    pub(crate) const fn coverage_producer(arguments: &'static [&'static str]) -> Self {
        Self {
            arguments,
            coverage_action: CoverageAction::Produce,
        }
    }

    pub(crate) const fn coverage_consumer(arguments: &'static [&'static str]) -> Self {
        Self {
            arguments,
            coverage_action: CoverageAction::Consume,
        }
    }

    pub fn arguments(self, execution: &GateExecutionEvidence) -> Vec<String> {
        let mut values = BAZEL_STARTUP_ARGUMENTS
            .iter()
            .map(|value| (*value).to_owned())
            .collect::<Vec<_>>();
        values.push(format!(
            "{BAZEL_OUTPUT_ROOT_PREFIX}{}",
            execution.output_user_root
        ));
        values.push(self.arguments[0].to_owned());
        values.push(format!(
            "{OVERRIDE_REPOSITORY_PREFIX}{}",
            execution.linux_artifact_repository
        ));
        values.push(format!(
            "{BAZEL_SYMLINK_PREFIX}{}",
            execution.symlink_prefix
        ));
        values.extend(self.arguments[1..].iter().map(|value| (*value).to_owned()));
        if self.consumes_coverage_report() {
            values.push(coverage_report_path(execution).display().to_string());
        }
        values
    }

    pub(crate) fn produces_coverage_report(self) -> bool {
        self.coverage_action == CoverageAction::Produce
    }

    pub(crate) fn consumes_coverage_report(self) -> bool {
        self.coverage_action == CoverageAction::Consume
    }
}

pub(crate) fn coverage_report_path(execution: &GateExecutionEvidence) -> PathBuf {
    Path::new(&execution.isolated_tmp)
        .join(RUST_COVERAGE_CUSTODY_DIRECTORY_NAME)
        .join(RUST_COVERAGE_CUSTODY_FILE_NAME)
}
