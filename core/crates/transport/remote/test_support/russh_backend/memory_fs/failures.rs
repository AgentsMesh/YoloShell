use super::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FailurePoint {
    CorruptUpload,
    UploadWrite,
    OversizedUploadWrite,
    FsyncFailure,
    FileClose,
    ReleaseRename,
    ActivationRename,
    ActivationResponseLoss,
    Permission,
    Metadata,
    StableMetadata,
    Mkdir,
    MkdirResponseLoss,
    UnsafeDirectory,
    TemporaryLinkTarget,
    ActivationWrongTarget,
    ActivationPreviousTarget,
    PosixUnexpectedPacket,
    SftpTransport,
    SftpInit,
    SftpLimitsRejected,
    SftpInitPending,
    SftpConfirmedOpenFailure,
    ChannelOpen,
    ExecReject,
    ExecDataBeforeApproval,
    ExecEofThenDataBeforeApproval,
    ExecFlowControlBeforeApproval,
    ExecDataWithoutApproval,
    SftpCanonicalize,
    SftpRead,
    BundleReadPending,
    SftpOpenResponsePending,
    SftpSymlinkResponsePending,
    SftpReadLinkHang,
    SftpSwapBeforeOpen,
    SftpSwapAfterOpen,
    SftpReplaceAfterOpen,
    SftpSwapAfterRead,
    SftpMetadataDriftAfterOpen,
    SftpFstatHang,
    SftpCloseHang,
}

impl MemoryFilesystem {
    pub fn take_transport_failure(&self) -> bool {
        self.take_failure(FailurePoint::SftpTransport)
    }

    pub fn take_confirmed_open_failure(&self) -> bool {
        self.take_failure(FailurePoint::SftpConfirmedOpenFailure)
    }

    pub fn take_channel_open_failure(&self) -> bool {
        self.take_failure(FailurePoint::ChannelOpen)
    }

    pub fn take_exec_rejection(&self) -> bool {
        self.take_failure(FailurePoint::ExecReject)
    }

    pub fn take_canonicalize_failure(&self) -> bool {
        self.take_failure(FailurePoint::SftpCanonicalize)
    }

    pub fn take_read_failure(&self) -> bool {
        self.take_failure(FailurePoint::SftpRead)
    }

    pub fn take_activation_response_loss(&self) -> bool {
        self.take_failure(FailurePoint::ActivationResponseLoss)
    }

    pub fn take_failure(&self, failure: FailurePoint) -> bool {
        let mut state = self.0.lock().unwrap();
        if state.failure != Some(failure) {
            return false;
        }
        state.failure = None;
        true
    }
}
