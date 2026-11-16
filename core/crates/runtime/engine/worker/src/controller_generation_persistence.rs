use super::{ControllerGeneration, ErrorBody, RuntimeRecord, WorkerState, lock_recover};
use crate::paths::identity::FileIdentity;
use crate::pty_custody::CustodyDurableRecord;
use std::sync::atomic::Ordering;

const STALE_GENERATION_DETAIL: &str = "controller generation is stale";
#[cfg(test)]
const INDETERMINATE_PERSISTENCE_DETAIL: &str =
    "injected indeterminate controller generation persistence failure";

impl WorkerState {
    pub(super) fn persist_custody_controller_generation(
        &self,
        requested: ControllerGeneration,
        custody: &CustodyDurableRecord,
        custody_identity: FileIdentity,
    ) -> std::result::Result<(), ErrorBody> {
        let current = self.controller_generation.load(Ordering::Acquire);
        let persisted = self.controller_persisted_generation.load(Ordering::Acquire);
        if requested.0 <= current || requested.0 <= persisted {
            return Err(ErrorBody::conflict(STALE_GENERATION_DETAIL));
        }
        let mut record = lock_recover(&self.record);
        #[cfg(test)]
        if self.control_test_hooks.take_controller_persist_failure() {
            return Err(ErrorBody::internal(
                super::CONTROLLER_PERSIST_FAILURE_TEST_DETAIL,
            ));
        }
        let result = crate::custody_promotion::publish_exact_with_boundary(
            self.namespace.lease(),
            &self.paths,
            custody,
            custody_identity,
            || {
                #[cfg(test)]
                self.control_test_hooks.run_after_custody_runtime_publish();
            },
            || {
                #[cfg(test)]
                anyhow::ensure!(
                    !self
                        .control_test_hooks
                        .take_controller_persist_indeterminate(),
                    INDETERMINATE_PERSISTENCE_DETAIL
                );
                Ok(())
            },
        );
        let published = result.map_err(persistence_error)?;
        if published.controller_generation != requested.0
            || published.validate_update_from(&record).is_err()
        {
            return Err(ErrorBody::internal(super::CONTROLLER_PERSISTENCE_CONTEXT));
        }
        *record = published;
        self.controller_persisted_generation
            .store(requested.0, Ordering::Release);
        self.controller_generation
            .store(requested.0, Ordering::Release);
        Ok(())
    }

    pub(super) fn persist_controller_generation(
        &self,
        requested: ControllerGeneration,
    ) -> std::result::Result<(), ErrorBody> {
        let current = self.controller_generation.load(Ordering::Acquire);
        if requested.0 < current {
            return Err(ErrorBody::conflict(format!(
                "{STALE_GENERATION_DETAIL}; requested {}, current {current}",
                requested.0
            )));
        }
        if self.controller_persisted_generation.load(Ordering::Acquire) >= requested.0 {
            return Ok(());
        }
        let mut record = lock_recover(&self.record);
        let mut next = record.clone();
        next.controller_generation = requested.0;
        #[cfg(test)]
        if self.control_test_hooks.take_controller_persist_failure() {
            return Err(ErrorBody::internal(
                super::CONTROLLER_PERSIST_FAILURE_TEST_DETAIL,
            ));
        }
        let result = self.persist_runtime_record(&next);
        #[cfg(test)]
        let result = result.and_then(|()| {
            anyhow::ensure!(
                !self
                    .control_test_hooks
                    .take_controller_persist_indeterminate(),
                INDETERMINATE_PERSISTENCE_DETAIL
            );
            Ok(())
        });
        if let Err(error) = result {
            self.reconcile_runtime_write(&next, error, super::CONTROLLER_PERSISTENCE_CONTEXT)?;
        }
        *record = next;
        self.controller_persisted_generation
            .store(requested.0, Ordering::Release);
        self.controller_generation
            .store(requested.0, Ordering::Release);
        Ok(())
    }

    pub(in crate::worker) fn reconcile_runtime_write(
        &self,
        expected: &RuntimeRecord,
        original: anyhow::Error,
        context: &str,
    ) -> std::result::Result<(), ErrorBody> {
        let path = self.paths.runtime_record(expected.terminal_id);
        let exact = self
            .namespace
            .read_json::<RuntimeRecord>(&path)
            .is_ok_and(|persisted| persisted == *expected);
        if exact {
            return Ok(());
        }
        Err(ErrorBody::internal(format!("{context}: {original}")))
    }
}

fn persistence_error(error: anyhow::Error) -> ErrorBody {
    ErrorBody::internal(format!(
        "{}: {error}",
        super::CONTROLLER_PERSISTENCE_CONTEXT
    ))
}
