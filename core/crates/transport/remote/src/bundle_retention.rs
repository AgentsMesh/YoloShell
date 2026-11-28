use super::BundleStageRequest;
use std::{
    fmt,
    path::{Path, PathBuf},
};
use uuid::Uuid;

pub const MAX_RETAINED_BUNDLE_STAGES: usize = 64;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BundleRetentionReason {
    Cancelled,
    Failed,
    Undelivered,
}

/// Diagnostic evidence of a possible upload residue, never filesystem deletion authority.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RetainedBundleStage {
    connection_id: Uuid,
    stage_id: Uuid,
    install_root: PathBuf,
    stage_directory: PathBuf,
    build_id: String,
    reason: BundleRetentionReason,
}

impl RetainedBundleStage {
    pub(super) fn new(connection_id: Uuid, request: &BundleStageRequest) -> Self {
        Self {
            connection_id,
            stage_id: request.stage_id(),
            install_root: request.install_root().to_owned(),
            stage_directory: request.stage_directory(),
            build_id: request.descriptor().build_id().to_owned(),
            reason: BundleRetentionReason::Undelivered,
        }
    }
    pub(super) fn with_reason(mut self, reason: BundleRetentionReason) -> Self {
        self.reason = reason;
        self
    }
    pub fn connection_id(&self) -> Uuid {
        self.connection_id
    }
    pub fn stage_id(&self) -> Uuid {
        self.stage_id
    }
    pub fn install_root(&self) -> &Path {
        &self.install_root
    }
    pub fn stage_directory(&self) -> &Path {
        &self.stage_directory
    }
    pub fn build_id(&self) -> &str {
        &self.build_id
    }
    pub fn reason(&self) -> BundleRetentionReason {
        self.reason
    }
}

impl fmt::Display for RetainedBundleStage {
    fn fmt(&self, output: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(output, "{}", self.stage_directory.display())
    }
}
