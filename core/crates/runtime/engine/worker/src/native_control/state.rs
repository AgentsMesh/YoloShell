use super::constants::*;
use crate::model::RuntimeRecord;
use crate::worker::{WorkerState, lock_recover};
use std::sync::atomic::Ordering;
use yolo_agent_control_runtime::{ControlRuntime, FreshInitialization};

impl WorkerState {
    pub(in crate::worker) fn initialize_native_control(&self, record: &RuntimeRecord) {
        let initialized = (|| -> anyhow::Result<ControlRuntime> {
            let session = self
                .namespace
                .lease()
                .directory(&self.paths.session_dir(record.terminal_id))?;
            let fresh = FreshInitialization::capture(session, &self.paths, record)?;
            Ok(ControlRuntime::initialize_fresh(fresh)?)
        })();
        match initialized {
            Ok(control) => {
                *lock_recover(&self.native_control) = Some(control);
                self.native_control_initialized
                    .store(true, Ordering::Release);
            }
            Err(_) => tracing::warn!("{INITIALIZATION_FAILED}"),
        }
    }

    #[cfg(test)]
    pub(in crate::worker) fn native_control_available(&self) -> bool {
        let record = lock_recover(&self.record).clone();
        let Ok(control) = self.native_control.try_lock() else {
            return false;
        };
        control
            .as_ref()
            .is_some_and(|control| control.verify(&record).is_ok())
    }
}
