use crate::{anchor::Anchor, constants::*, descriptor::io_error, prepared::PreparedRead};
use std::path::PathBuf;
use yolo_hub_file_protocol::{FileRootSummary, ReadFileRequest, ReadFileResult};
use yolo_hub_terminal_protocol::PublicError;

pub struct ReadRoot {
    pub(crate) summary: FileRootSummary,
    pub(crate) anchor: Anchor,
    pub(crate) protected: Vec<Anchor>,
}

impl ReadRoot {
    /// Opens existing directories only; it never normalizes their modes or ACLs.
    pub fn open(summary: FileRootSummary, protected: &[PathBuf]) -> Result<Self, PublicError> {
        if protected.is_empty() || protected.len() > MAX_PROTECTED_ROOTS {
            return Err(PublicError::Unauthorized);
        }
        let anchor = Anchor::absolute(&summary.fields().path)?;
        let protected = protected
            .iter()
            .map(|path| {
                let canonical = path.canonicalize().map_err(io_error)?;
                Anchor::absolute(canonical.to_str().ok_or(PublicError::InvalidMetadata)?)
            })
            .collect::<Result<Vec<_>, PublicError>>()?;
        let root = Self {
            summary,
            anchor,
            protected,
        };
        root.verify()?;
        Ok(root)
    }

    pub fn summary(&self) -> &FileRootSummary {
        &self.summary
    }

    pub fn fingerprint(&self) -> Result<yolo_crypto::Sha256Digest, PublicError> {
        self.verify()?;
        yolo_crypto::Sha256Digest::capture(&(
            FILE_ROOT_IDENTITY_DOMAIN,
            self.anchor.object_identity()?,
        ))
        .map_err(|_| PublicError::Internal)
    }

    pub fn prepare(&self, request: &ReadFileRequest) -> Result<PreparedRead, PublicError> {
        if request.fields().target != self.summary.fields().target {
            return Err(PublicError::TargetMismatch);
        }
        self.verify()?;
        PreparedRead::open(self, request)
    }

    /// Blocking I/O belongs behind a supervised process boundary; a timeout cannot cancel a syscall.
    pub fn read(&self, request: &ReadFileRequest) -> Result<ReadFileResult, PublicError> {
        self.prepare(request)?.read()
    }

    pub(crate) fn verify(&self) -> Result<(), PublicError> {
        self.anchor.verify()?;
        for protected in &self.protected {
            protected.verify()?;
            if self.anchor.overlaps(protected) {
                return Err(PublicError::Unauthorized);
            }
        }
        Ok(())
    }
}
