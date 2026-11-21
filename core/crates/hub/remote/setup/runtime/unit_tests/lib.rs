use crate::{Job, constants::*, task_retention};
use std::collections::BTreeMap;
use tokio::{sync::watch, time::Instant};
use uuid::Uuid;
use yolo_hub_remote_setup_protocol::{RemoteSetupOutcome, RemoteSetupStatus};

mod broker_directory;
mod diagnostics;

const COMPLETION_STEP: std::time::Duration = std::time::Duration::from_millis(1);
const EXTRA_COMPLETIONS: usize = 3;

fn job(outcome: RemoteSetupOutcome, completed_at: Option<Instant>, admitted_at: Instant) -> Job {
    let status = RemoteSetupStatus {
        request: yolo_hub_remote_setup_protocol_test_support::fixture::request(),
        outcome,
    };
    let (_, receiver) = watch::channel(status);
    Job {
        handle: tokio::spawn(std::future::pending()),
        status: receiver,
        admitted_at,
        completed_at,
    }
}

#[tokio::test]
async fn terminal_results_have_an_independent_bound_and_active_jobs_are_never_evicted() {
    let now = Instant::now();
    let mut jobs = BTreeMap::new();
    let active = (0..MAX_ACTIVE_SETUPS)
        .map(|_| {
            let id = Uuid::new_v4();
            jobs.insert(id, job(RemoteSetupOutcome::Running {}, None, now));
            id
        })
        .collect::<Vec<_>>();
    let mut terminals = Vec::new();
    for index in 0..MAX_RETAINED_SETUP_RESULTS + EXTRA_COMPLETIONS {
        let id = Uuid::new_v4();
        let completed = now + COMPLETION_STEP * u32::try_from(index).unwrap();
        jobs.insert(
            id,
            job(RemoteSetupOutcome::Unknown {}, Some(completed), now),
        );
        terminals.push(id);
    }
    task_retention::prune(&mut jobs);
    assert_eq!(jobs.len(), MAX_ACTIVE_SETUPS + MAX_RETAINED_SETUP_RESULTS);
    assert!(active.iter().all(|id| jobs.contains_key(id)));
    assert!(
        terminals[..EXTRA_COMPLETIONS]
            .iter()
            .all(|id| !jobs.contains_key(id))
    );
    assert!(
        terminals[EXTRA_COMPLETIONS..]
            .iter()
            .all(|id| jobs.contains_key(id))
    );
    assert_eq!(
        jobs.values().filter(|job| !job.completed()).count(),
        MAX_ACTIVE_SETUPS
    );
}

#[tokio::test]
async fn terminated_task_with_no_outcome_is_unknown_instead_of_running_forever() {
    let now = Instant::now();
    let job = job(RemoteSetupOutcome::Running {}, None, now);
    job.handle.abort();
    while !job.handle.is_finished() {
        tokio::task::yield_now().await;
    }
    assert!(job.completed());
    assert!(matches!(
        job.snapshot().outcome,
        RemoteSetupOutcome::Unknown {}
    ));
}
