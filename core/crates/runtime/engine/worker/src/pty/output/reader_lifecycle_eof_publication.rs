use super::KernelPtyEofIdentity;
use crate::worker::ReaderLifecycle;
use crate::worker::reader::reader_lifecycle::KernelPtyEofRead;
use std::sync::Arc;

pub(crate) struct EofPublication<'a> {
    pub(crate) lifecycle: &'a ReaderLifecycle,
    pub(crate) identity: Arc<KernelPtyEofIdentity>,
    pub(crate) completed: bool,
}

impl<'a> KernelPtyEofRead<'a> {
    pub(crate) fn await_eof_publication(self) -> EofPublication<'a> {
        let mut attempt = self.into_attempt();
        let mut progress = attempt.lifecycle.lock_progress();
        let probe = attempt.kernel_eof_probe.take();
        let identity = publication_identity(probe, progress.kernel_eof_probe_identity.as_ref());
        progress.kernel_eof_probe_identity = None;
        progress.eof.await_publication(Arc::clone(&identity));
        drop(progress);
        attempt.completed = true;
        EofPublication {
            lifecycle: attempt.lifecycle,
            identity,
            completed: false,
        }
    }
}

pub(crate) fn publication_identity(
    probe: Option<Arc<KernelPtyEofIdentity>>,
    current_probe: Option<&Arc<KernelPtyEofIdentity>>,
) -> Arc<KernelPtyEofIdentity> {
    let probe = probe
        .filter(|identity| current_probe.is_some_and(|current| Arc::ptr_eq(current, identity)));
    probe.unwrap_or_else(KernelPtyEofIdentity::from_kernel_read)
}

impl EofPublication<'_> {
    pub(crate) fn is_current(&self) -> bool {
        self.lifecycle.validates_kernel_eof_identity(&self.identity)
    }

    pub(crate) fn publish(mut self) {
        let mut progress = self.lifecycle.lock_progress();
        progress.eof.publish(&self.identity);
        progress.output_read_in_flight = false;
        self.completed = true;
        self.lifecycle.changed.notify_all();
    }
}

impl Drop for EofPublication<'_> {
    fn drop(&mut self) {
        if self.completed {
            return;
        }
        let mut progress = self.lifecycle.lock_progress();
        progress.eof.retain(&self.identity);
        progress.output_read_in_flight = false;
        self.lifecycle.changed.notify_all();
    }
}

#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-unit-tests"))]
#[cfg(any(feature = "yolo-integration-tests", feature = "yolo-worker-unit-runtime"))]
#[path = "../../../unit_tests/reader/reader_lifecycle_eof_publication_tests.rs"]
mod tests;
