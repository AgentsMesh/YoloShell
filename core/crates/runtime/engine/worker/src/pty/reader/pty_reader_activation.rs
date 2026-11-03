use super::pty_output_progress::{ExclusivePtyOutput, ReaderBackedPtyOutput};
use super::{PTY_READER_THREAD_PREFIX, WorkerState, supervise_reader};
use anyhow::anyhow;
use anyhow::{Context, Result};
use std::io::Read;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::sync::mpsc::{SyncSender, sync_channel};

pub(crate) const ACTIVATION_CHANNEL_CAPACITY: usize = 1;
pub(crate) const READER_SPAWN_CONTEXT: &str = "spawn PTY reader thread";
pub(crate) const READER_ACTIVATION_DETAIL: &str = "staged PTY reader stopped before activation";
pub(crate) const PREPARED_READER_INVARIANT: &str =
    "prepared PTY reader remains blocked on its activation channel";

pub(crate) struct PendingPtyReader {
    pub(crate) activation: SyncSender<()>,
    pub(crate) lifecycle: Arc<super::ReaderLifecycle>,
    pub(crate) child_waited: Arc<AtomicBool>,
    pub(crate) runtime: Arc<super::pty_runtime_alias_custody::PtyRuntimeAliasCustody>,
    #[cfg(test)]
    pub(crate) activation_barrier: Option<Arc<super::control_test_hooks::BlockingControlHook>>,
}

pub(crate) struct PreparedPtyReader {
    pub(crate) activation: SyncSender<()>,
    pub(crate) lifecycle: Arc<super::ReaderLifecycle>,
    pub(crate) child_waited: Arc<AtomicBool>,
    pub(crate) runtime: Arc<super::pty_runtime_alias_custody::PtyRuntimeAliasCustody>,
    #[cfg(test)]
    pub(crate) activation_barrier: Option<Arc<super::control_test_hooks::BlockingControlHook>>,
}

#[cfg(test)]
pub(crate) struct ReaderActivationFailure {
    pub(crate) error: anyhow::Error,
    pub(crate) output: ExclusivePtyOutput,
}

impl PendingPtyReader {
    pub(crate) fn prepare(self, output: &mut ExclusivePtyOutput) -> Result<PreparedPtyReader> {
        output.prepare_reader_activation()?;
        Ok(PreparedPtyReader {
            activation: self.activation,
            lifecycle: self.lifecycle,
            child_waited: self.child_waited,
            runtime: self.runtime,
            #[cfg(test)]
            activation_barrier: self.activation_barrier,
        })
    }

    #[cfg(test)]
    pub(crate) fn activate(
        self,
        mut output: ExclusivePtyOutput,
    ) -> std::result::Result<ReaderBackedPtyOutput, ReaderActivationFailure> {
        let prepared = match self.prepare(&mut output) {
            Ok(prepared) => prepared,
            Err(error) => return Err(ReaderActivationFailure { error, output }),
        };
        prepared.activate(output)
    }

    #[cfg(test)]
    pub(crate) fn activate_without_output(self) -> Result<()> {
        self.activation
            .send(())
            .map_err(|_| anyhow!(READER_ACTIVATION_DETAIL))
    }
}

impl PreparedPtyReader {
    #[cfg(test)]
    pub(crate) fn activate(
        self,
        output: ExclusivePtyOutput,
    ) -> std::result::Result<ReaderBackedPtyOutput, ReaderActivationFailure> {
        self.activate_output(output)
            .map_err(|(error, output)| ReaderActivationFailure { error, output })
    }

    pub(crate) fn activate_infallible(self, output: ExclusivePtyOutput) -> ReaderBackedPtyOutput {
        self.activate_output(output)
            .unwrap_or_else(|_| unreachable!("{PREPARED_READER_INVARIANT}"))
    }

    pub(crate) fn activate_output(
        self,
        output: ExclusivePtyOutput,
    ) -> std::result::Result<ReaderBackedPtyOutput, (anyhow::Error, ExclusivePtyOutput)> {
        let Self {
            activation,
            lifecycle,
            child_waited,
            runtime,
            #[cfg(test)]
            activation_barrier,
        } = self;
        output.activate_runtime_reader(lifecycle, child_waited, runtime, || {
            #[cfg(test)]
            if let Some(barrier) = activation_barrier {
                barrier.pause_if_armed();
            }
            activation
                .send(())
                .map_err(|_| anyhow!(READER_ACTIVATION_DETAIL))
        })
    }
}

pub(crate) fn stage_reader(
    reader: Box<dyn Read + Send>,
    state: Arc<WorkerState>,
) -> Result<PendingPtyReader> {
    let (activation, receiver) = sync_channel(ACTIVATION_CHANNEL_CAPACITY);
    let worker = Arc::clone(&state);
    std::thread::Builder::new()
        .name(format!(
            "{}-{}",
            PTY_READER_THREAD_PREFIX,
            state.terminal_id()
        ))
        .spawn(move || {
            if receiver.recv().is_ok() {
                supervise_reader(reader, worker);
            }
        })
        .context(READER_SPAWN_CONTEXT)?;
    Ok(PendingPtyReader {
        activation,
        lifecycle: Arc::clone(&state.reader_lifecycle),
        child_waited: Arc::clone(&state.child_waited),
        runtime: Arc::clone(&state.pty_alias_custody),
        #[cfg(test)]
        activation_barrier: None,
    })
}

#[cfg(test)]
pub(crate) fn spawn_reader(reader: Box<dyn Read + Send>, state: Arc<WorkerState>) -> Result<()> {
    stage_reader(reader, state)?.activate_without_output()
}

#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-integration-tests"))]
#[path = "../../../integration_tests/pty/reader/pty_reader_activation_tests.rs"]
mod tests;

#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-integration-tests"))]
#[path = "../../../integration_tests/pty/reader/pty_reader_eof_handoff_adversarial_tests.rs"]
mod eof_handoff_adversarial_tests;

#[cfg(test)]
impl PendingPtyReader {
    pub(crate) fn for_test() -> (Self, std::sync::mpsc::Receiver<()>) {
        let (activation, receiver) = sync_channel(ACTIVATION_CHANNEL_CAPACITY);
        (
            Self {
                activation,
                lifecycle: Arc::new(super::ReaderLifecycle::default()),
                child_waited: Arc::new(AtomicBool::new(false)),
                runtime: Arc::new(
                    super::pty_runtime_alias_custody::PtyRuntimeAliasCustody::new_for_test(),
                ),
                activation_barrier: None,
            },
            receiver,
        )
    }

    pub(crate) fn disconnected_for_test() -> Self {
        let (reader, receiver) = Self::for_test();
        drop(receiver);
        reader
    }

    pub(crate) fn lifecycle_for_test(&self) -> Arc<super::ReaderLifecycle> {
        Arc::clone(&self.lifecycle)
    }

    pub(crate) fn install_activation_barrier_for_test(
        &mut self,
        barrier: Arc<super::control_test_hooks::BlockingControlHook>,
    ) {
        self.activation_barrier = Some(barrier);
    }
}
