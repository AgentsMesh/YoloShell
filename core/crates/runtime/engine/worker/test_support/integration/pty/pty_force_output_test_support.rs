use crate::worker::ReaderLifecycle;
use crate::worker::pty_output_progress::OutputProgressState;
use anyhow::bail;

const SCRIPT_FENCE_MISMATCH_DETAIL: &str = "scripted output received another fence kind";
const BEGIN_FAILURE_REUSED_DETAIL: &str = "begin-fence failure reached the poll operation";
const OPERATION_CALL_INCREMENT: usize = 1;
pub(super) enum FenceBehavior {
    BeginError(&'static str),
    PollError(&'static str),
    Pending,
    Reached,
}

pub(super) enum ProgressBehavior {
    Error(&'static str),
    Pending,
    Complete,
}

enum FinalEofRevalidationFault {
    Revoke,
    Error(&'static str),
    Panic(&'static str),
}

pub(in crate::worker) struct ScriptedForceOutput {
    fence: FenceBehavior,
    fence_poll_panic: Option<&'static str>,
    reach_hook: Option<Box<dyn FnOnce()>>,
    report_panic: Option<&'static str>,
    report_error: Option<&'static str>,
    progress: ProgressBehavior,
    kernel_eof: std::sync::Arc<ReaderLifecycle>,
    eof_authority_version_override: Option<ProcessClosureVersion>,
    final_eof_revalidation_fault: Option<FinalEofRevalidationFault>,
    final_eof_revalidation_calls: usize,
    final_eof_token_mints: usize,
    eof_acquire_calls: usize,
    begin_calls: usize,
    poll_calls: usize,
    report_calls: usize,
    progress_calls: usize,
    pub(super) master_release_delivered: bool,
}

impl ScriptedForceOutput {
    fn new(fence: FenceBehavior) -> Self {
        Self {
            fence,
            fence_poll_panic: None,
            reach_hook: None,
            report_panic: None,
            report_error: None,
            progress: ProgressBehavior::Pending,
            kernel_eof: std::sync::Arc::new(ReaderLifecycle::default()),
            eof_authority_version_override: None,
            final_eof_revalidation_fault: None,
            final_eof_revalidation_calls: usize::default(),
            final_eof_token_mints: usize::default(),
            eof_acquire_calls: usize::default(),
            begin_calls: usize::default(),
            poll_calls: usize::default(),
            report_calls: usize::default(),
            progress_calls: usize::default(),
            master_release_delivered: false,
        }
    }

    fn with_report_error(mut self, detail: &'static str) -> Self {
        self.report_error = Some(detail);
        self
    }

    fn with_progress_error(mut self, detail: &'static str) -> Self {
        self.progress = ProgressBehavior::Error(detail);
        self
    }

    fn set_fence(&mut self, fence: FenceBehavior) {
        self.fence = fence;
    }

    fn set_progress(&mut self, progress: ProgressBehavior) {
        self.progress = progress;
    }

    fn observe_kernel_eof(&self) {
        self.kernel_eof.mark_eof_observed();
    }

    fn revoke_eof_during_final_revalidation(&mut self) {
        self.final_eof_revalidation_fault = Some(FinalEofRevalidationFault::Revoke);
    }

    fn fail_after_final_eof_token_mint(&mut self, detail: &'static str) {
        self.final_eof_revalidation_fault = Some(FinalEofRevalidationFault::Error(detail));
    }

    fn panic_after_final_eof_token_mint(&mut self, detail: &'static str) {
        self.final_eof_revalidation_fault = Some(FinalEofRevalidationFault::Panic(detail));
    }

    fn panic_on_next_fence_poll(&mut self, detail: &'static str) {
        self.fence_poll_panic = Some(detail);
    }

    fn run_when_fence_reaches(&mut self, operation: impl FnOnce() + 'static) {
        self.reach_hook = Some(Box::new(operation));
    }

    fn panic_on_next_report(&mut self, detail: &'static str) {
        self.report_panic = Some(detail);
    }

    pub(super) fn begin_fence(&mut self) -> Result<OutputFence> {
        self.begin_calls += OPERATION_CALL_INCREMENT;
        match self.fence {
            FenceBehavior::BeginError(detail) => bail!(detail),
            _ => Ok(OutputFence::Scripted),
        }
    }

    pub(super) fn poll_fence(&mut self, fence: &mut OutputFence) -> Result<()> {
        self.poll_calls += OPERATION_CALL_INCREMENT;
        if let Some(detail) = self.fence_poll_panic.take() {
            panic!("{detail}");
        }
        if !matches!(fence, OutputFence::Scripted) {
            bail!(SCRIPT_FENCE_MISMATCH_DETAIL);
        }
        match self.fence {
            FenceBehavior::BeginError(_) => bail!(BEGIN_FAILURE_REUSED_DETAIL),
            FenceBehavior::PollError(detail) => bail!(detail),
            FenceBehavior::Pending => Ok(()),
            FenceBehavior::Reached => {
                if let Some(operation) = self.reach_hook.take() {
                    operation();
                }
                *fence = OutputFence::Reached;
                Ok(())
            }
        }
    }

    pub(super) fn report_producers_gone(&mut self) -> Result<()> {
        self.report_calls += OPERATION_CALL_INCREMENT;
        if let Some(detail) = self.report_panic.take() {
            panic!("{detail}");
        }
        self.report_error
            .map_or(Ok(()), |detail| Err(anyhow::anyhow!(detail)))
    }

    pub(super) fn progress(&mut self) -> Result<OutputProgressState> {
        self.progress_calls += OPERATION_CALL_INCREMENT;
        match self.progress {
            ProgressBehavior::Error(detail) => bail!(detail),
            ProgressBehavior::Pending => Ok(OutputProgressState::Pending),
            ProgressBehavior::Complete => Ok(OutputProgressState::Complete),
        }
    }

    pub(super) fn acquire_kernel_eof_authority(
        &mut self,
        version: &ProcessClosureVersion,
    ) -> Option<KernelPtyEofAuthority> {
        self.eof_acquire_calls += OPERATION_CALL_INCREMENT;
        let version = self
            .eof_authority_version_override
            .as_ref()
            .unwrap_or(version);
        KernelPtyEofAuthority::scripted_reader(version, &self.kernel_eof)
    }

    pub(super) fn validates_kernel_eof_authority(
        &mut self,
        authority: &KernelPtyEofAuthority,
        version: &ProcessClosureVersion,
    ) -> Result<bool> {
        Ok(authority.validates_scripted_reader(version, &self.kernel_eof))
    }
}
