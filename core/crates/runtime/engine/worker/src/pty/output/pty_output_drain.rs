use super::{
    ExclusivePtyOutput, OutputDescriptor, OutputProgressState, OutputQueueState, PtyOutputRecovery,
};
use crate::worker::ReaderLifecycle;
use crate::worker::reader::reader_lifecycle::OutputProbeEpoch;
use anyhow::Result;
use std::sync::Arc;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum OutputQuiescenceState {
    Pending,
    Reached,
}

pub(crate) enum OutputQuiescenceFence {
    Immediate,
    AwaitingQueue {
        lifecycle: Arc<ReaderLifecycle>,
    },
    AwaitingReader {
        lifecycle: Arc<ReaderLifecycle>,
        epoch: OutputProbeEpoch,
    },
}

pub(crate) enum ExclusiveOutputQuiescenceFence {
    AwaitingQueue,
    AwaitingFinalRead,
}

impl ExclusivePtyOutput {
    pub(crate) fn begin_quiescence_fence() -> ExclusiveOutputQuiescenceFence {
        ExclusiveOutputQuiescenceFence::AwaitingQueue
    }

    pub fn poll_quiescence_fence(
        &mut self,
        fence: &mut ExclusiveOutputQuiescenceFence,
    ) -> Result<OutputQuiescenceState> {
        let progress = self.progress()?;
        let queue = self.output_queue_state()?;
        match fence {
            ExclusiveOutputQuiescenceFence::AwaitingQueue => {
                if queue == OutputQueueState::Empty {
                    *fence = ExclusiveOutputQuiescenceFence::AwaitingFinalRead;
                }
                Ok(OutputQuiescenceState::Pending)
            }
            ExclusiveOutputQuiescenceFence::AwaitingFinalRead => {
                if queue == OutputQueueState::Pending {
                    *fence = ExclusiveOutputQuiescenceFence::AwaitingQueue;
                    return Ok(OutputQuiescenceState::Pending);
                }
                Ok(if progress == OutputProgressState::Pending {
                    OutputQuiescenceState::Pending
                } else {
                    OutputQuiescenceState::Reached
                })
            }
        }
    }

    pub fn output_queue_state(&self) -> Result<OutputQueueState> {
        match &self.descriptor {
            OutputDescriptor::Native(file) => {
                super::super::pty_output_terminal::output_queue_state(file)
            }
            OutputDescriptor::Released => Ok(OutputQueueState::Empty),
            #[cfg(test)]
            OutputDescriptor::Observed(_) | OutputDescriptor::Quiescent(_) => {
                Ok(OutputQueueState::Empty)
            }
        }
    }
}

impl PtyOutputRecovery {
    pub(crate) fn begin_quiescence_fence(&mut self) -> Result<OutputQuiescenceFence> {
        match &*self.descriptor.lock() {
            OutputDescriptor::Native(file) => super::super::pty_output_terminal::resume(file)?,
            OutputDescriptor::Released => return Ok(OutputQuiescenceFence::Immediate),
            #[cfg(test)]
            OutputDescriptor::Observed(observed) | OutputDescriptor::Quiescent(observed) => {
                observed.fetch_add(1, std::sync::atomic::Ordering::AcqRel);
            }
        }
        let Some(lifecycle) = self.reader_lifecycle.as_ref().map(Arc::clone) else {
            return Ok(OutputQuiescenceFence::Immediate);
        };
        lifecycle.wake_output_reader();
        Ok(OutputQuiescenceFence::AwaitingQueue { lifecycle })
    }

    pub fn poll_quiescence_fence(
        &self,
        fence: &mut OutputQuiescenceFence,
    ) -> Result<OutputQuiescenceState> {
        if matches!(fence, OutputQuiescenceFence::Immediate) {
            return Ok(OutputQuiescenceState::Reached);
        }
        if fence.output_was_abandoned() {
            return Ok(OutputQuiescenceState::Reached);
        }
        fence.observe_queue(self.output_queue_state()?)
    }

    pub fn output_queue_state(&self) -> Result<OutputQueueState> {
        match &*self.descriptor.lock() {
            OutputDescriptor::Native(file) => {
                super::super::pty_output_terminal::output_queue_state(file)
            }
            OutputDescriptor::Released => Ok(OutputQueueState::Empty),
            #[cfg(test)]
            OutputDescriptor::Observed(_) | OutputDescriptor::Quiescent(_) => {
                Ok(OutputQueueState::Empty)
            }
        }
    }
}

impl OutputQuiescenceFence {
    pub(crate) fn output_was_abandoned(&self) -> bool {
        match self {
            Self::Immediate => false,
            Self::AwaitingQueue { lifecycle } | Self::AwaitingReader { lifecycle, .. } => {
                lifecycle.output_progress_state() == OutputProgressState::Abandoned
            }
        }
    }

    pub(crate) fn observe_queue(
        &mut self,
        queue: OutputQueueState,
    ) -> Result<OutputQuiescenceState> {
        Ok(match self {
            OutputQuiescenceFence::Immediate => OutputQuiescenceState::Reached,
            OutputQuiescenceFence::AwaitingQueue { lifecycle } => {
                if queue == OutputQueueState::Pending {
                    return Ok(OutputQuiescenceState::Pending);
                }
                let epoch = lifecycle.request_output_probe()?;
                *self = OutputQuiescenceFence::AwaitingReader {
                    lifecycle: Arc::clone(lifecycle),
                    epoch,
                };
                OutputQuiescenceState::Pending
            }
            OutputQuiescenceFence::AwaitingReader { lifecycle, epoch } => {
                if queue == OutputQueueState::Pending {
                    lifecycle.wake_output_reader();
                    *self = OutputQuiescenceFence::AwaitingQueue {
                        lifecycle: Arc::clone(lifecycle),
                    };
                    return Ok(OutputQuiescenceState::Pending);
                }
                let state = lifecycle.output_quiescence_state(*epoch);
                if state == OutputQuiescenceState::Pending {
                    lifecycle.wake_output_reader();
                }
                state
            }
        })
    }
}

#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-integration-tests"))]
#[path = "../../../integration_tests/pty/output/pty_output_drain_tests.rs"]
mod tests;
