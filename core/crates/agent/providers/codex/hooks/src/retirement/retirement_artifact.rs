use super::{CodexRetirementRecord, CodexRetirementSeal};
use crate::paths::identity::FileIdentity;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CodexRetirementSealArtifact {
    record: CodexRetirementSeal,
    file_identity: FileIdentity,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CodexRetirementArtifact {
    record: CodexRetirementRecord,
    file_identity: FileIdentity,
}

pub struct CodexRetirementSealPublication {
    artifact: CodexRetirementSealArtifact,
    replay: bool,
}

pub struct CodexRetirementPublication {
    artifact: CodexRetirementArtifact,
    replay: bool,
}

impl CodexRetirementSealPublication {
    pub(super) fn new(record: CodexRetirementSeal, identity: FileIdentity, replay: bool) -> Self {
        Self {
            artifact: CodexRetirementSealArtifact {
                record,
                file_identity: identity,
            },
            replay,
        }
    }

    pub fn artifact(&self) -> &CodexRetirementSealArtifact {
        &self.artifact
    }

    pub fn is_replay(&self) -> bool {
        self.replay
    }
}

impl CodexRetirementSealArtifact {
    pub(super) fn from_exact(record: CodexRetirementSeal, file_identity: FileIdentity) -> Self {
        Self {
            record,
            file_identity,
        }
    }

    pub fn record(&self) -> &CodexRetirementSeal {
        &self.record
    }

    pub fn file_identity(&self) -> FileIdentity {
        self.file_identity
    }

    pub(super) fn exact(&self) -> (CodexRetirementSeal, FileIdentity) {
        (self.record.clone(), self.file_identity)
    }
}

impl CodexRetirementPublication {
    pub(super) fn new(record: CodexRetirementRecord, identity: FileIdentity, replay: bool) -> Self {
        Self {
            artifact: CodexRetirementArtifact {
                record,
                file_identity: identity,
            },
            replay,
        }
    }

    pub fn artifact(&self) -> &CodexRetirementArtifact {
        &self.artifact
    }

    pub fn is_replay(&self) -> bool {
        self.replay
    }
}

impl CodexRetirementArtifact {
    pub(super) fn from_exact(record: CodexRetirementRecord, file_identity: FileIdentity) -> Self {
        Self {
            record,
            file_identity,
        }
    }

    pub(crate) fn record(&self) -> &CodexRetirementRecord {
        &self.record
    }

    pub fn file_identity(&self) -> FileIdentity {
        self.file_identity
    }

    pub(super) fn exact(&self) -> (CodexRetirementRecord, FileIdentity) {
        (self.record.clone(), self.file_identity)
    }
}
