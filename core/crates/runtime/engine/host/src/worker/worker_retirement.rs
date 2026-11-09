pub(crate) enum WorkerRetirementError {
    Control {
        phase: WorkerRetirementControlPhase,
        error: WorkerRpcError,
    },
    ExitTimeout,
    SocketServing,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum WorkerRetirementControlPhase {
    CustodiedRpc,
    ReleasedEvidenceBeforeAuth,
    Authentication,
    ReleasedEvidenceAfterAuth,
    ShutdownEncoding,
    ShutdownExchange,
    ShutdownResponse,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum WorkerRetirementControlFailure {
    Transport,
    OutcomeUnknown,
    Protocol,
    IdentityMismatch,
    WorkerShuttingDown,
    RemoteRejected,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum WorkerRetirementObservation {
    NotRequested,
    Completed,
    Control {
        phase: WorkerRetirementControlPhase,
        failure: WorkerRetirementControlFailure,
    },
    AcknowledgedExitTimeout,
    AcknowledgedSocketServing,
}

impl WorkerRetirementError {
    pub(crate) fn control(phase: WorkerRetirementControlPhase, error: WorkerRpcError) -> Self {
        Self::Control { phase, error }
    }

    pub(crate) fn observation(&self) -> WorkerRetirementObservation {
        match self {
            Self::Control { phase, error } => WorkerRetirementObservation::Control {
                phase: *phase,
                failure: WorkerRetirementControlFailure::classify(error),
            },
            Self::ExitTimeout => WorkerRetirementObservation::AcknowledgedExitTimeout,
            Self::SocketServing => WorkerRetirementObservation::AcknowledgedSocketServing,
        }
    }

    pub(crate) fn may_settle_after_disconnect(&self) -> bool {
        match self {
            Self::Control { error, .. } => {
                error.is_transport()
                    || matches!(error, WorkerRpcError::OutcomeUnknown)
                    || matches!(
                        error, WorkerRpcError::Remote(body)
                            if body.code == crate::protocol::error_code::WORKER_SHUTTING_DOWN
                    )
            }
            Self::ExitTimeout => true,
            Self::SocketServing => false,
        }
    }

    pub(crate) fn into_error_body(self) -> ErrorBody {
        match self {
            Self::Control { error, .. } => error.into_error_body(),
            Self::ExitTimeout => worker_unreachable(WORKER_RETIREMENT_EXIT_TIMEOUT),
            Self::SocketServing => worker_unreachable(WORKER_RETIREMENT_SOCKET_ACTIVE),
        }
    }
}

impl WorkerRetirementControlFailure {
    pub(crate) fn classify(error: &WorkerRpcError) -> Self {
        match error {
            WorkerRpcError::Closed | WorkerRpcError::Transport(_) => Self::Transport,
            WorkerRpcError::OutcomeUnknown => Self::OutcomeUnknown,
            WorkerRpcError::Protocol(_) => Self::Protocol,
            WorkerRpcError::IdentityMismatch(_) => Self::IdentityMismatch,
            WorkerRpcError::Remote(body)
                if body.code == crate::protocol::error_code::WORKER_SHUTTING_DOWN =>
            {
                Self::WorkerShuttingDown
            }
            WorkerRpcError::Remote(_) => Self::RemoteRejected,
        }
    }
}
