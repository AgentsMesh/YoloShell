//! Turn the Host catalogue watch into Hub invalidation and publication
//! wakeups.
//!
//! The stream carries revision-stamped hints, never catalogue views. A hint
//! drops the shared catalogue snapshot so the next read is exact; a revision
//! gap (first connect, Host restart, lag) performs the same invalidation, so
//! a missed hint can only delay a refresh until the next hint or heartbeat.

use std::time::Duration;
use tokio::sync::watch;
use yolo_hub_local_backend::LocalBackend;

const INITIAL_BACKOFF: Duration = Duration::from_millis(100);
const MAX_BACKOFF: Duration = Duration::from_secs(5);

pub async fn run(
    backend: LocalBackend,
    wake: watch::Sender<u64>,
    mut stop: watch::Receiver<bool>,
) -> anyhow::Result<()> {
    let mut revision = 0u64;
    let mut backoff = INITIAL_BACKOFF;
    while !*stop.borrow() {
        match watch_once(&backend, &wake, &mut revision, &mut stop).await {
            Ok(()) => break,
            Err(_) => {
                backend.invalidate_catalogue().await;
                bump(&wake);
                tokio::select! {
                    _ = stop.changed() => break,
                    _ = tokio::time::sleep(backoff) => {}
                }
                backoff = (backoff * 2).min(MAX_BACKOFF);
            }
        }
    }
    Ok(())
}

async fn watch_once(
    backend: &LocalBackend,
    wake: &watch::Sender<u64>,
    revision: &mut u64,
    stop: &mut watch::Receiver<bool>,
) -> Result<(), yolo_client::ClientError> {
    let mut watch = backend.watch_catalogue(*revision).await?;
    if watch.start_revision != *revision {
        backend.invalidate_catalogue().await;
        bump(wake);
    }
    *revision = watch.start_revision;
    loop {
        tokio::select! {
            _ = stop.changed() => return Ok(()),
            change = watch.next_change() => {
                let change = change?;
                *revision = (*revision).max(change.revision);
                backend.invalidate_catalogue().await;
                bump(wake);
            }
        }
    }
}

fn bump(wake: &watch::Sender<u64>) {
    let next = *wake.borrow() + 1;
    wake.send_replace(next);
}

#[cfg(all(test, feature = "yolo-integration-tests"))]
#[path = "../integration_tests/lib.rs"]
mod integration_tests;
