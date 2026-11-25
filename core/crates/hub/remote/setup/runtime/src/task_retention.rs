use crate::{Job, constants::MAX_RETAINED_SETUP_RESULTS};
use std::{
    collections::BTreeMap,
    sync::{Mutex, Weak},
};
use tokio::time::Instant;
use uuid::Uuid;
use yolo_hub_remote_setup_protocol::{RemoteSetupOutcome, RemoteSetupStatus};

impl Job {
    pub(crate) fn completed(&self) -> bool {
        self.completed_at.is_some() || self.handle.is_finished()
    }
    pub(crate) fn snapshot(&self) -> RemoteSetupStatus {
        let mut status = self.status.borrow().clone();
        if self.handle.is_finished() && matches!(status.outcome, RemoteSetupOutcome::Running {}) {
            status.outcome = RemoteSetupOutcome::Unknown {};
        }
        status
    }
}

pub(crate) fn finished(jobs: Weak<Mutex<BTreeMap<Uuid, Job>>>, id: Uuid) {
    let Some(jobs) = jobs.upgrade() else {
        return;
    };
    let Ok(mut jobs) = jobs.lock() else {
        return;
    };
    if let Some(job) = jobs.get_mut(&id) {
        job.completed_at = Some(Instant::now());
    }
    prune(&mut jobs);
}

/// Completed results are observations only. Never evict active mutation custody.
pub(crate) fn prune(jobs: &mut BTreeMap<Uuid, Job>) {
    let mut completed = jobs
        .iter()
        .filter(|(_, job)| job.completed())
        .map(|(id, job)| (job.completed_at.unwrap_or(job.admitted_at), *id))
        .collect::<Vec<_>>();
    completed.sort_unstable();
    let excess = completed.len().saturating_sub(MAX_RETAINED_SETUP_RESULTS);
    for (_, id) in completed.into_iter().take(excess) {
        jobs.remove(&id);
    }
}
