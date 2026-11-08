#[path = "../test_support/fixture.rs"]
mod fixture;

use std::collections::VecDeque;
use std::sync::Mutex;
use std::time::Duration;

use fixture::{STREAM, idle, promptable, request, respondable, snapshot, target, until_input};
use tokio::sync::watch;
use yolo_hub_agent_protocol::{
    Activity, AgentSnapshot, Lifecycle, ObservationCursor,
};
use crate::{AgentSnapshots, SnapshotFuture, WaitSnapshot, wait};
use yolo_hub_terminal_protocol::PublicError;

struct Script {
    sweeps: Mutex<VecDeque<WaitSnapshot>>,
}

impl Script {
    fn new(sweeps: Vec<Vec<AgentSnapshot>>) -> Self {
        Self::with_current(
            sweeps
                .into_iter()
                .map(|agents| {
                    let current = agents.iter().map(|agent| agent.cursor().target()).collect();
                    WaitSnapshot { agents, current }
                })
                .collect(),
        )
    }

    fn with_current(sweeps: Vec<WaitSnapshot>) -> Self {
        Self {
            sweeps: Mutex::new(sweeps.into()),
        }
    }
}

impl AgentSnapshots for Script {
    fn snapshots(&self) -> SnapshotFuture<'_> {
        let snapshot = {
            let mut sweeps = self.sweeps.lock().unwrap();
            if sweeps.len() > 1 {
                sweeps.pop_front().unwrap_or_default()
            } else {
                sweeps.front().cloned().unwrap_or_default()
            }
        };
        Box::pin(async move { Ok(snapshot) })
    }
}


fn deadline(ms: u64) -> tokio::time::Instant {
    tokio::time::Instant::now() + Duration::from_millis(ms)
}

fn bump_after(tx: watch::Sender<u64>, ms: u64) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        tokio::time::sleep(Duration::from_millis(ms)).await;
        let next = *tx.borrow() + 1;
        tx.send_replace(next);
    })
}

fn working(target: yolo_hub_identity::AgentTarget) -> AgentSnapshot {
    snapshot(target, Activity::Working, Lifecycle::Running {}, 2)
}

#[tokio::test]
async fn an_already_matching_state_returns_immediately() {
    let target = target();
    let script = Script::new(vec![vec![idle(target)]]);
    let (_tx, rx) = watch::channel(0u64);
    let result = wait(
        &script,
        target.address.hub_id,
        &request(vec![target], until_input(), None, 1_000),
        rx,
        deadline(1_000),
    )
    .await
    .unwrap();
    assert_eq!(result.matched.len(), 1);
    assert_eq!(result.matched[0].cursor().target(), target);
    assert!(result.ended.is_empty());
    assert!(!result.timed_out);
}

#[tokio::test]
async fn respondable_matches_only_an_answerable_interaction() {
    let target = target();
    let script = Script::new(vec![vec![respondable(target)]]);
    let (_tx, rx) = watch::channel(0u64);
    let result = wait(
        &script,
        target.address.hub_id,
        &request(
            vec![target],
            yolo_hub_agent_protocol::AgentWaitCondition::Respondable,
            None,
            1_000,
        ),
        rx,
        deadline(1_000),
    )
    .await
    .unwrap();
    assert_eq!(result.matched.len(), 1);
    assert!(!result.timed_out);

    let script = Script::new(vec![vec![idle(target)]]);
    let (_tx, rx) = watch::channel(0u64);
    let result = wait(
        &script,
        target.address.hub_id,
        &request(
            vec![target],
            yolo_hub_agent_protocol::AgentWaitCondition::Respondable,
            None,
            200,
        ),
        rx,
        deadline(40),
    )
    .await
    .unwrap();
    assert!(result.matched.is_empty());
    assert!(result.timed_out);
}

#[tokio::test]
async fn a_catalogue_hint_wakes_the_wait() {
    let target = target();
    let script = Script::new(vec![vec![working(target)], vec![idle(target)]]);
    let (tx, rx) = watch::channel(0u64);
    let bump = bump_after(tx, 20);
    let started = tokio::time::Instant::now();
    let result = wait(
        &script,
        target.address.hub_id,
        &request(vec![target], until_input(), None, 1_000),
        rx,
        deadline(800),
    )
    .await
    .unwrap();
    bump.abort();
    assert_eq!(result.matched.len(), 1);
    assert!(started.elapsed() < Duration::from_millis(500));
}

#[tokio::test]
async fn a_deadline_reports_a_timeout() {
    let target = target();
    let script = Script::new(vec![vec![working(target)]]);
    let (_tx, rx) = watch::channel(0u64);
    let result = wait(
        &script,
        target.address.hub_id,
        &request(vec![target], until_input(), None, 200),
        rx,
        deadline(40),
    )
    .await
    .unwrap();
    assert!(result.matched.is_empty());
    assert!(result.timed_out);
}

#[tokio::test]
async fn a_replaced_execution_ends_instead_of_waiting_out() {
    let target = target();
    // History keeps the old snapshot; only the current-execution set drops it.
    let script = Script::with_current(vec![
        WaitSnapshot {
            agents: vec![working(target)],
            current: vec![target],
        },
        WaitSnapshot {
            agents: vec![working(target)],
            current: vec![],
        },
    ]);
    let (tx, rx) = watch::channel(0u64);
    let bump = bump_after(tx, 20);
    let result = wait(
        &script,
        target.address.hub_id,
        &request(vec![target], until_input(), None, 800),
        rx,
        deadline(800),
    )
    .await
    .unwrap();
    bump.abort();
    assert!(result.matched.is_empty());
    assert_eq!(result.ended, vec![target]);
    assert!(!result.timed_out);
}

#[tokio::test]
async fn a_cursor_requires_a_newer_observation() {
    let target = target();
    let script = Script::new(vec![vec![idle(target)]]);
    let after = ObservationCursor::new(target, STREAM, 2).unwrap();
    let (_tx, rx) = watch::channel(0u64);
    let result = wait(
        &script,
        target.address.hub_id,
        &request(vec![target], until_input(), Some(after), 200),
        rx,
        deadline(40),
    )
    .await
    .unwrap();
    assert!(result.timed_out);

    let after = ObservationCursor::new(target, STREAM, 1).unwrap();
    let (_tx, rx) = watch::channel(0u64);
    let result = wait(
        &script,
        target.address.hub_id,
        &request(vec![target], until_input(), Some(after), 1_000),
        rx,
        deadline(200),
    )
    .await
    .unwrap();
    assert_eq!(result.matched.len(), 1);
}

#[tokio::test]
async fn a_promptable_condition_requires_an_available_capability() {
    use yolo_hub_agent_protocol::AgentWaitCondition;
    let target = target();
    let script = Script::new(vec![vec![idle(target)]]);
    let (_tx, rx) = watch::channel(0u64);
    let result = wait(
        &script,
        target.address.hub_id,
        &request(vec![target], AgentWaitCondition::Promptable, None, 200),
        rx,
        deadline(40),
    )
    .await
    .unwrap();
    assert!(result.timed_out);

    let script = Script::new(vec![vec![promptable(target)]]);
    let (_tx, rx) = watch::channel(0u64);
    let result = wait(
        &script,
        target.address.hub_id,
        &request(vec![target], AgentWaitCondition::Promptable, None, 1_000),
        rx,
        deadline(200),
    )
    .await
    .unwrap();
    assert_eq!(result.matched.len(), 1);
}

#[tokio::test]
async fn a_foreign_target_is_unsupported() {
    let target = target();
    let script = Script::new(vec![vec![idle(target)]]);
    let (_tx, rx) = watch::channel(0u64);
    let error = wait(
        &script,
        yolo_hub_identity::HubId::new(),
        &request(vec![target], until_input(), None, 1_000),
        rx,
        deadline(200),
    )
    .await
    .unwrap_err();
    assert!(matches!(error, PublicError::Unsupported));
}

#[tokio::test]
async fn an_unknown_target_is_not_found() {
    let target = target();
    let script = Script::new(vec![vec![]]);
    let (_tx, rx) = watch::channel(0u64);
    let error = wait(
        &script,
        target.address.hub_id,
        &request(vec![target], until_input(), None, 1_000),
        rx,
        deadline(200),
    )
    .await
    .unwrap_err();
    assert!(matches!(error, PublicError::NotFound));
}
