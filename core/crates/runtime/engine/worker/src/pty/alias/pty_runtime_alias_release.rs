use super::super::secondary_custody_authority::ReadyWorkerCustodyIdentity;
use super::PtyRuntimeAliasCustody;
use crate::process::{PtyHostReleaseVerifier, PtyLocalMasterAliasesClosed, PtyMasterAliasesClosed};
use crate::worker::lock_recover;
use anyhow::Result;
use std::sync::mpsc;

pub(crate) const HOST_RELEASE_DISCONNECTED_DETAIL: &str =
    "Host PTY custody release completion channel disconnected";
pub(crate) const HOST_RELEASE_STATE_CHANGED_DETAIL: &str =
    "PTY Host release state changed while its transition lock was held";

pub(crate) enum HostReleaseState {
    Open,
    Retryable(RetryableHostRelease),
    Awaiting {
        retryable: RetryableHostRelease,
        completion:
            mpsc::Receiver<super::super::secondary_custody_release::WorkerCustodyReleaseResult>,
    },
    Finalized,
}

pub(crate) struct RetryableHostRelease {
    pub(crate) local: PtyLocalMasterAliasesClosed,
    pub(crate) verifier: PtyHostReleaseVerifier,
    pub(crate) last_attempt: Option<ReadyWorkerCustodyIdentity>,
}

pub(crate) enum PtyRuntimeLocalAliasRelease {
    Pending,
    Closed(PtyLocalMasterAliasesClosed, PtyHostReleaseVerifier),
}

impl PtyRuntimeLocalAliasRelease {
    pub(crate) fn closed(proof: (PtyLocalMasterAliasesClosed, PtyHostReleaseVerifier)) -> Self {
        Self::Closed(proof.0, proof.1)
    }
}

pub(crate) enum PtyRuntimeAliasReleasePoll {
    Pending,
    Closed(PtyMasterAliasesClosed),
    Delivered,
}

impl PtyRuntimeAliasCustody {
    pub(crate) fn release<Close>(&self, close: Close) -> Result<PtyRuntimeAliasReleasePoll>
    where
        Close: FnOnce() -> Result<PtyRuntimeLocalAliasRelease>,
    {
        let mut state = lock_recover(&self.host_release);
        match &mut *state {
            HostReleaseState::Open => self.begin_release(&mut state, close),
            HostReleaseState::Retryable(_) => self.begin_attempt(&mut state),
            HostReleaseState::Awaiting { .. } => Self::poll_completion(&mut state),
            HostReleaseState::Finalized => Ok(PtyRuntimeAliasReleasePoll::Delivered),
        }
    }

    pub(crate) fn begin_release<Close>(
        &self,
        state: &mut HostReleaseState,
        close: Close,
    ) -> Result<PtyRuntimeAliasReleasePoll>
    where
        Close: FnOnce() -> Result<PtyRuntimeLocalAliasRelease>,
    {
        let (local, verifier) = match close() {
            Ok(PtyRuntimeLocalAliasRelease::Pending) => {
                return Ok(PtyRuntimeAliasReleasePoll::Pending);
            }
            Ok(PtyRuntimeLocalAliasRelease::Closed(local, verifier)) => (local, verifier),
            Err(error) => {
                *state = HostReleaseState::Open;
                return Err(error);
            }
        };
        *state = HostReleaseState::Retryable(RetryableHostRelease {
            local,
            verifier,
            last_attempt: None,
        });
        self.begin_attempt(state)
    }

    pub(crate) fn begin_attempt(
        &self,
        state: &mut HostReleaseState,
    ) -> Result<PtyRuntimeAliasReleasePoll> {
        let Some(handle) = lock_recover(&self.release_handle).clone() else {
            return Ok(PtyRuntimeAliasReleasePoll::Pending);
        };
        let identity = handle.identity();
        let retryable = retryable_mut(state);
        if retryable.last_attempt == Some(identity) {
            return Ok(PtyRuntimeAliasReleasePoll::Pending);
        }
        retryable.last_attempt = Some(identity);
        match handle.begin(retryable.verifier.clone()) {
            Ok(completion) => {
                let retryable = take_retryable(state);
                *state = HostReleaseState::Awaiting {
                    retryable,
                    completion,
                };
                Ok(PtyRuntimeAliasReleasePoll::Pending)
            }
            Err(error) => super::release_retry::after_error(error),
        }
    }

    pub(crate) fn poll_completion(
        state: &mut HostReleaseState,
    ) -> Result<PtyRuntimeAliasReleasePoll> {
        let completion = match state {
            HostReleaseState::Awaiting { completion, .. } => completion,
            _ => unreachable!("{HOST_RELEASE_STATE_CHANGED_DETAIL}"),
        };
        match completion.try_recv() {
            Ok(Ok(acknowledgement)) => {
                let (retryable, _completion) = take_awaiting(state);
                let RetryableHostRelease {
                    local,
                    verifier,
                    last_attempt,
                } = retryable;
                let finalized = local
                    .finalize_host_release(acknowledgement)
                    .map(|settlement| settlement.mint_final_proof());
                match finalized {
                    Ok(proof) => {
                        *state = HostReleaseState::Finalized;
                        Ok(PtyRuntimeAliasReleasePoll::Closed(proof))
                    }
                    Err((local, error)) => {
                        *state = HostReleaseState::Retryable(RetryableHostRelease {
                            local,
                            verifier,
                            last_attempt,
                        });
                        super::release_retry::after_error(error)
                    }
                }
            }
            Ok(Err(failure)) => {
                let (retryable, _completion) = take_awaiting(state);
                *state = HostReleaseState::Retryable(retryable);
                super::release_retry::after_detail(failure.detail())
            }
            Err(mpsc::TryRecvError::Empty) => Ok(PtyRuntimeAliasReleasePoll::Pending),
            Err(mpsc::TryRecvError::Disconnected) => {
                let (retryable, _completion) = take_awaiting(state);
                *state = HostReleaseState::Retryable(retryable);
                super::release_retry::after_detail(HOST_RELEASE_DISCONNECTED_DETAIL)
            }
        }
    }
}

pub(crate) fn retryable_mut(state: &mut HostReleaseState) -> &mut RetryableHostRelease {
    match state {
        HostReleaseState::Retryable(retryable) => retryable,
        _ => unreachable!("{HOST_RELEASE_STATE_CHANGED_DETAIL}"),
    }
}

pub(crate) fn take_retryable(state: &mut HostReleaseState) -> RetryableHostRelease {
    match std::mem::replace(state, HostReleaseState::Finalized) {
        HostReleaseState::Retryable(retryable) => retryable,
        unexpected => restore_unexpected(state, unexpected),
    }
}

pub(crate) fn take_awaiting(
    state: &mut HostReleaseState,
) -> (
    RetryableHostRelease,
    mpsc::Receiver<super::super::secondary_custody_release::WorkerCustodyReleaseResult>,
) {
    match std::mem::replace(state, HostReleaseState::Finalized) {
        HostReleaseState::Awaiting {
            retryable,
            completion,
        } => (retryable, completion),
        unexpected => restore_unexpected(state, unexpected),
    }
}

pub(crate) fn restore_unexpected<T>(
    state: &mut HostReleaseState,
    unexpected: HostReleaseState,
) -> T {
    *state = unexpected;
    unreachable!("{HOST_RELEASE_STATE_CHANGED_DETAIL}")
}
