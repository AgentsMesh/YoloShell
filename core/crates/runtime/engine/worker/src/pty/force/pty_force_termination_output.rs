use super::{ForceOutput, OUTPUT_FENCE_KIND_CHANGED_DETAIL, OutputFence};
use crate::process::TrackedProducersGone;
use crate::worker::pty_output_progress::{
    ExclusivePtyOutput, OutputProgressState, OutputQuiescenceState,
};
use anyhow::{Result, bail};

impl ForceOutput<'_> {
    pub(crate) fn begin_fence(&mut self) -> Result<OutputFence> {
        match self {
            Self::Reader(output) => output.begin_quiescence_fence().map(OutputFence::Reader),
            Self::Exclusive(_) => Ok(OutputFence::Exclusive(
                ExclusivePtyOutput::begin_quiescence_fence(),
            )),
            #[cfg(test)]
            Self::Scripted(output) => output.begin_fence(),
        }
    }

    pub(crate) fn poll_fence(&mut self, fence: &mut OutputFence) -> Result<()> {
        let state = match self {
            Self::Reader(output) => match fence {
                OutputFence::Reader(fence) => output.poll_quiescence_fence(fence)?,
                OutputFence::Reached => return Ok(()),
                _ => bail!(OUTPUT_FENCE_KIND_CHANGED_DETAIL),
            },
            Self::Exclusive(output) => match fence {
                OutputFence::Exclusive(fence) => output.poll_quiescence_fence(fence)?,
                OutputFence::Reached => return Ok(()),
                _ => bail!(OUTPUT_FENCE_KIND_CHANGED_DETAIL),
            },
            #[cfg(test)]
            Self::Scripted(output) => return output.poll_fence(fence),
        };
        if state == OutputQuiescenceState::Reached {
            *fence = OutputFence::Reached;
        }
        Ok(())
    }

    pub(crate) fn report_producers_gone(&mut self, proof: &TrackedProducersGone) -> Result<()> {
        match self {
            Self::Reader(output) => output.report_tracked_producers_gone(proof),
            Self::Exclusive(output) => output.report_tracked_producers_gone(proof),
            #[cfg(test)]
            Self::Scripted(output) => output.report_producers_gone(),
        }
    }

    pub(crate) fn progress(&mut self) -> Result<OutputProgressState> {
        match self {
            Self::Reader(output) => output.progress(),
            Self::Exclusive(output) => output.progress(),
            #[cfg(test)]
            Self::Scripted(output) => output.progress(),
        }
    }
}
#[cfg(target_os = "linux")]
impl ForceOutput<'_> {
    pub(super) fn prepare_linux_hangup(
        &self,
        tree: &mut crate::process::FreezingProcessTree,
    ) -> anyhow::Result<Option<crate::process::LinuxPtyHangup>> {
        use crate::worker::pty_output_progress::OutputDescriptor;
        let guard;
        let (descriptor, binding) = match self {
            Self::Reader(output) => {
                guard = output.descriptor.lock();
                (&*guard, output.master_alias_authority.as_ref())
            }
            Self::Exclusive(output) => (&output.descriptor, output.master_alias_authority.as_ref()),
            #[cfg(test)]
            Self::Scripted(_) => return Ok(None),
        };
        match (descriptor, binding) {
            (OutputDescriptor::Native(file), Some(binding)) => {
                tree.prepare_linux_hangup(file, binding)
            }
            _ => Ok(None),
        }
    }
}
