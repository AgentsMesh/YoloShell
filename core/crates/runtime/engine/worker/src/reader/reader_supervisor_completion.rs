use super::ReaderLifecycle;
use std::io::Read;
use std::sync::Arc;

#[derive(Debug)]
pub(crate) struct ReaderSupervisorCompletion {
    _private: (),
}

pub(crate) struct ReaderSupervisorCustody {
    initial_reader: Option<Box<dyn Read + Send>>,
    lifecycle: Arc<ReaderLifecycle>,
}

impl ReaderSupervisorCustody {
    pub(crate) fn new(
        initial_reader: Box<dyn Read + Send>,
        lifecycle: Arc<ReaderLifecycle>,
    ) -> Self {
        Self {
            initial_reader: Some(initial_reader),
            lifecycle,
        }
    }

    pub(crate) fn take_initial(&mut self) -> Option<Box<dyn Read + Send>> {
        self.initial_reader.take()
    }
}

impl Drop for ReaderSupervisorCustody {
    fn drop(&mut self) {
        drop(self.initial_reader.take());
        self.lifecycle
            .publish_supervisor_completion(ReaderSupervisorCompletion { _private: () });
    }
}

impl ReaderLifecycle {
    fn publish_supervisor_completion(&self, completion: ReaderSupervisorCompletion) {
        let mut progress = self.lock_progress();
        if progress.reader_supervisor_completion.is_none() {
            progress.reader_supervisor_completion = Some(completion);
        }
        self.changed.notify_all();
    }

    #[cfg(test)]
    pub(crate) fn mark_supervisor_completed_for_test(&self) {
        self.publish_supervisor_completion(ReaderSupervisorCompletion { _private: () });
    }
}
