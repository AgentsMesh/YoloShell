use super::{AgentDetector, DetectionResult, ObservedProcess};
use anyhow::Result;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum AgentScanError {
    #[error("process scan failed: {0}")]
    Process(#[source] anyhow::Error),
    #[error("agent detection failed: {0}")]
    Detection(#[source] anyhow::Error),
}

impl AgentScanError {
    pub fn code(&self) -> &'static str {
        match self {
            Self::Process(_) => super::constants::error::PROCESS_SCAN_FAILED,
            Self::Detection(_) => super::constants::error::DETECTION_FAILED,
        }
    }
}

pub fn detect_scan(
    detector: &mut AgentDetector,
    processes: Result<&[ObservedProcess]>,
) -> std::result::Result<DetectionResult, AgentScanError> {
    let processes = processes.map_err(AgentScanError::Process)?;
    detector
        .detect(processes)
        .map_err(AgentScanError::Detection)
}

#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-unit-tests"))]
#[path = "../unit_tests/scan/tests.rs"]
mod tests;
