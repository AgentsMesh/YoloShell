use super::super::constants::{
    MAX_PTY_PROOF_CANDIDATES, MAX_PTY_PROOF_TOTAL_ENVIRONMENT_BYTES,
    MAX_PTY_PROOF_TOTAL_FILE_DESCRIPTORS,
};
use anyhow::anyhow;
use std::error::Error;
use std::fmt::{self, Display, Formatter};

const CANDIDATE_BUDGET_DETAIL: &str = "PTY proof candidate inventory exceeds its budget";
const DESCRIPTOR_BUDGET_DETAIL: &str = "PTY proof aggregate descriptor work exceeds its budget";
const ENVIRONMENT_BUDGET_DETAIL: &str = "PTY proof aggregate environment work exceeds its budget";
const ENVIRONMENT_ITEM_BUDGET_DETAIL: &str = "PTY proof process environment exceeds its budget";
const CANDIDATE_QUERY_KIND: &str = "PTY proof candidate-local query failed";
const DESCRIPTOR_SCAN_DEFERRED_KIND: &str = "PTY proof descriptor scan deferred";
const DESCRIPTOR_SCAN_DEFERRED_DETAIL: &str = "child spawn descriptor inheritance is unsettled";
const CANDIDATE_IDENTITY_CONTEXT: &str = "scan PTY proof candidate";
const HOLDER_QUERY_KIND: &str = "PTY proof path-holder inventory query failed";
const SCAN_BUDGET_KIND: &str = "PTY proof scan budget exceeded";
const BATCH_BUDGET_KIND: &str = "PTY proof scan batch exhausted";
const INVARIANT_KIND: &str = "PTY proof response invariant failed";

#[derive(Debug)]
pub(super) enum CapabilityProofError {
    CandidateLocalQuery(anyhow::Error),
    DescriptorScanDeferred(anyhow::Error),
    HolderInventoryQuery(anyhow::Error),
    ScanBudgetExceeded(anyhow::Error),
    BatchBudgetExhausted(anyhow::Error),
    Invariant(anyhow::Error),
}

impl CapabilityProofError {
    pub(super) fn candidate(error: impl Into<anyhow::Error>) -> Self {
        Self::CandidateLocalQuery(error.into())
    }

    pub(super) fn descriptor_scan_deferred() -> Self {
        Self::DescriptorScanDeferred(anyhow!(DESCRIPTOR_SCAN_DEFERRED_DETAIL))
    }

    pub(super) fn budget(error: impl Into<anyhow::Error>) -> Self {
        Self::ScanBudgetExceeded(error.into())
    }

    pub(super) fn holder_query(error: impl Into<anyhow::Error>) -> Self {
        Self::HolderInventoryQuery(error.into())
    }

    fn batch(error: impl Into<anyhow::Error>) -> Self {
        Self::BatchBudgetExhausted(error.into())
    }

    pub(super) fn invariant(error: impl Into<anyhow::Error>) -> Self {
        Self::Invariant(error.into())
    }

    pub(super) fn with_candidate_identity(self, pid: u32, generation: u64) -> Self {
        let context = format!("{CANDIDATE_IDENTITY_CONTEXT}: pid={pid}, generation={generation}");
        match self {
            Self::CandidateLocalQuery(error) => Self::CandidateLocalQuery(error.context(context)),
            Self::DescriptorScanDeferred(error) => {
                Self::DescriptorScanDeferred(error.context(context))
            }
            Self::HolderInventoryQuery(error) => Self::HolderInventoryQuery(error.context(context)),
            Self::ScanBudgetExceeded(error) => Self::ScanBudgetExceeded(error.context(context)),
            Self::BatchBudgetExhausted(error) => Self::BatchBudgetExhausted(error.context(context)),
            Self::Invariant(error) => Self::Invariant(error.context(context)),
        }
    }

    fn parts(&self) -> (&'static str, &anyhow::Error) {
        match self {
            Self::CandidateLocalQuery(error) => (CANDIDATE_QUERY_KIND, error),
            Self::DescriptorScanDeferred(error) => (DESCRIPTOR_SCAN_DEFERRED_KIND, error),
            Self::HolderInventoryQuery(error) => (HOLDER_QUERY_KIND, error),
            Self::ScanBudgetExceeded(error) => (SCAN_BUDGET_KIND, error),
            Self::BatchBudgetExhausted(error) => (BATCH_BUDGET_KIND, error),
            Self::Invariant(error) => (INVARIANT_KIND, error),
        }
    }
}

impl Display for CapabilityProofError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        let (kind, error) = self.parts();
        write!(formatter, "{kind}: {error:#}")
    }
}

impl Error for CapabilityProofError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        Some(self.parts().1.as_ref())
    }
}

pub(super) type CapabilityProofResult<T> = Result<T, CapabilityProofError>;

#[derive(Clone, Copy)]
enum BudgetMode {
    Atomic,
    Batched,
}

pub(super) struct CapabilityScanBudget {
    descriptors: usize,
    environment_bytes: usize,
    environment_buffer: Vec<std::mem::MaybeUninit<u8>>,
    mode: BudgetMode,
}

impl CapabilityScanBudget {
    pub(super) fn start(candidates: usize) -> CapabilityProofResult<Self> {
        if candidates > MAX_PTY_PROOF_CANDIDATES {
            return Err(CapabilityProofError::budget(anyhow!(
                CANDIDATE_BUDGET_DETAIL
            )));
        }
        Ok(Self::new(BudgetMode::Atomic))
    }

    pub(super) fn start_batch() -> Self {
        Self::new(BudgetMode::Batched)
    }

    pub(super) fn admit_descriptors(&mut self, count: usize) -> CapabilityProofResult<()> {
        let total = self
            .descriptors
            .checked_add(count)
            .filter(|total| *total <= MAX_PTY_PROOF_TOTAL_FILE_DESCRIPTORS);
        let total = total.ok_or_else(|| self.aggregate_error(DESCRIPTOR_BUDGET_DETAIL))?;
        self.descriptors = total;
        Ok(())
    }

    pub(super) fn admit_environment(&mut self, count: usize) -> CapabilityProofResult<()> {
        Self::validate_environment_item(count)?;
        self.add_environment(count)
    }

    fn add_environment(&mut self, count: usize) -> CapabilityProofResult<()> {
        let total = self
            .environment_bytes
            .checked_add(count)
            .filter(|total| *total <= MAX_PTY_PROOF_TOTAL_ENVIRONMENT_BYTES);
        let total = total.ok_or_else(|| self.aggregate_error(ENVIRONMENT_BUDGET_DETAIL))?;
        self.environment_bytes = total;
        Ok(())
    }

    fn new(mode: BudgetMode) -> Self {
        Self {
            descriptors: usize::default(),
            environment_bytes: usize::default(),
            environment_buffer: Vec::new(),
            mode,
        }
    }

    fn aggregate_error(&self, detail: &'static str) -> CapabilityProofError {
        match self.mode {
            BudgetMode::Atomic => CapabilityProofError::budget(anyhow!(detail)),
            BudgetMode::Batched => CapabilityProofError::batch(anyhow!(detail)),
        }
    }

    #[cfg(test)]
    pub(super) fn environment_bytes(&self) -> usize {
        self.environment_bytes
    }

    #[cfg(test)]
    pub(super) fn descriptor_count(&self) -> usize {
        self.descriptors
    }
}

#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-integration-tests"))]
#[path = "../../../../integration_tests/pty/session/pty_session_batch_budget_tests.rs"]
mod batch_tests;
#[path = "../pty_session_environment_budget_macos.rs"]
mod environment_budget;
