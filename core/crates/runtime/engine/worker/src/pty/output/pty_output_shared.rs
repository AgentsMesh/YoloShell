use super::OutputDescriptor;
use crate::worker::pty_runtime_alias_custody::PtyRuntimeAliasCustody;
use std::sync::{Arc, Mutex};

pub(crate) struct SharedOutputDescriptor {
    pub(crate) descriptor: Mutex<OutputDescriptor>,
    pub(crate) runtime: Option<Arc<PtyRuntimeAliasCustody>>,
}

impl SharedOutputDescriptor {
    pub(crate) fn new(
        descriptor: OutputDescriptor,
        runtime: Option<Arc<PtyRuntimeAliasCustody>>,
    ) -> Self {
        Self {
            descriptor: Mutex::new(descriptor),
            runtime,
        }
    }

    pub(crate) fn lock(&self) -> std::sync::MutexGuard<'_, OutputDescriptor> {
        self.descriptor
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    pub(crate) fn release(&self) {
        let descriptor = std::mem::replace(&mut *self.lock(), OutputDescriptor::Released);
        drop(descriptor);
    }
}
