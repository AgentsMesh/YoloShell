use super::BundleStageRequest;
use super::bundle_retention::{
    BundleRetentionReason, MAX_RETAINED_BUNDLE_STAGES, RetainedBundleStage,
};

const STAGE_RECORD_LIMIT: &str =
    "bundle staging records are full or this stage already requires inspection";
const RETAINED_MESSAGE: &str =
    "unproven bundle stage retained for native inspection; this record does not authorize deletion";

struct StageEntry {
    report: RetainedBundleStage,
    attempted: bool,
    retained: bool,
}

pub(super) struct StageRetention {
    registry: Option<RemoteCleanupRegistry>,
    stage: uuid::Uuid,
}

impl RemoteCleanupRegistry {
    pub fn retained_bundle_stages(&self) -> Vec<RetainedBundleStage> {
        self.inner
            .state
            .lock()
            .unwrap()
            .stages
            .values()
            .filter(|entry| entry.retained)
            .map(|entry| entry.report.clone())
            .collect()
    }

    pub(super) fn reserve_stage(
        &self,
        connection: uuid::Uuid,
        request: &BundleStageRequest,
    ) -> Result<StageRetention, RemoteError> {
        let mut state = self.inner.state.lock().unwrap();
        if state.phase != RegistryPhase::Open {
            return Err(RemoteError::transport(CLEANUP_REGISTRY_CLOSED_MESSAGE));
        }
        if state.stages.len() >= MAX_RETAINED_BUNDLE_STAGES
            || state.stages.contains_key(&request.stage_id())
        {
            return Err(RemoteError::RuntimeDeployment(STAGE_RECORD_LIMIT.into()));
        }
        state.stages.insert(
            request.stage_id(),
            StageEntry {
                report: RetainedBundleStage::new(connection, request),
                attempted: false,
                retained: false,
            },
        );
        Ok(StageRetention {
            registry: Some(self.clone()),
            stage: request.stage_id(),
        })
    }
}

impl StageRetention {
    pub fn attempted(&self) {
        if let Some(registry) = &self.registry {
            if let Some(entry) = registry
                .inner
                .state
                .lock()
                .unwrap()
                .stages
                .get_mut(&self.stage)
            {
                entry.attempted = true;
            }
        }
    }
    pub fn accepted(mut self) {
        if let Some(registry) = self.registry.take() {
            registry
                .inner
                .state
                .lock()
                .unwrap()
                .stages
                .remove(&self.stage);
        }
    }
    pub fn retain(mut self, reason: BundleRetentionReason) -> Option<RetainedBundleStage> {
        self.finish(reason)
    }
    fn finish(&mut self, reason: BundleRetentionReason) -> Option<RetainedBundleStage> {
        let registry = self.registry.take()?;
        let mut state = registry.inner.state.lock().unwrap();
        let mut entry = state.stages.remove(&self.stage)?;
        if !entry.attempted {
            return None;
        }
        entry.retained = true;
        entry.report = entry.report.with_reason(reason);
        let report = entry.report.clone();
        state.stages.insert(self.stage, entry);
        drop(state);
        tracing::warn!(stage = %report, message = RETAINED_MESSAGE);
        Some(report)
    }
}

impl Drop for StageRetention {
    fn drop(&mut self) {
        let _ = self.finish(BundleRetentionReason::Cancelled);
    }
}
