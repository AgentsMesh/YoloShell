//! Push-driven long-poll over typed agent observations.
//!
//! The waiter re-reads the Hub projection on every catalogue change hint and
//! on a slow rescan tick. A condition only counts once its target shows a
//! state newer than the caller's cursor, and an execution that is replaced or
//! removed while waiting is reported instead of waiting out the deadline.

use std::future::Future;
use std::pin::Pin;
use std::time::Duration;
use tokio::sync::watch;
use tokio::time::Instant;
use yolo_hub_agent_protocol::{
    AgentSnapshot, AgentWaitRequest, AgentWaitResult, ObservationCursor,
};
use yolo_hub_identity::{AgentTarget, HubId};
use yolo_hub_terminal_protocol::PublicError;

mod prompt;
pub use prompt::prompt_turn;

/// How often a wait refreshes without a catalogue change hint. The Host feed
/// covers binding and worker events; this only guards missed hints.
const RESCAN: Duration = Duration::from_secs(1);

pub type SnapshotFuture<'a> =
    Pin<Box<dyn Future<Output = Result<WaitSnapshot, PublicError>> + Send + 'a>>;

/// One evaluation input: every projected execution plus the executions that
/// are current for their terminal. A pinned target that is no longer current
/// was replaced or removed and cannot satisfy a wait.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct WaitSnapshot {
    pub agents: Vec<AgentSnapshot>,
    pub current: Vec<AgentTarget>,
}

/// The projection a wait evaluates. The Hub backend implements it; tests use
/// a scripted sequence.
pub trait AgentSnapshots {
    fn snapshots(&self) -> SnapshotFuture<'_>;
}

impl AgentSnapshots for yolo_hub_local_backend::LocalBackend {
    fn snapshots(&self) -> SnapshotFuture<'_> {
        Box::pin(async move {
            let agents = self.observe_agents().await?;
            let terminal_agents = self.terminals(false).await?;
            let current = terminal_agents
                .iter()
                .filter_map(|terminal| terminal.fields().agent)
                .collect();
            Ok(WaitSnapshot { agents, current })
        })
    }
}

/// Evaluate `request` against `source` until a target matches, every target
/// ended, or `deadline` passes. `wake` carries catalogue change hints.
pub async fn wait<S: AgentSnapshots>(
    source: &S,
    local: HubId,
    request: &AgentWaitRequest,
    mut wake: watch::Receiver<u64>,
    deadline: Instant,
) -> Result<AgentWaitResult, PublicError> {
    request
        .validate()
        .map_err(|_| PublicError::InvalidRequest)?;
    if request
        .targets
        .iter()
        .any(|target| target.address.hub_id != local)
    {
        return Err(PublicError::Unsupported);
    }
    let mut first = true;
    loop {
        // Consume hints received before this sweep; a hint arriving during the
        // sweep still wakes the next iteration.
        wake.borrow_and_update();
        let snapshot = source.snapshots().await?;
        let evaluation = evaluate(request, &snapshot, first)?;
        first = false;
        if !evaluation.matched.is_empty() {
            return Ok(AgentWaitResult {
                targets: request.targets.clone(),
                matched: evaluation.matched,
                ended: evaluation.ended,
                timed_out: false,
            });
        }
        if evaluation.ended.len() == request.targets.len() {
            return Ok(AgentWaitResult {
                targets: request.targets.clone(),
                matched: Vec::new(),
                ended: evaluation.ended,
                timed_out: false,
            });
        }
        let now = Instant::now();
        if now >= deadline {
            return Ok(AgentWaitResult {
                targets: request.targets.clone(),
                matched: Vec::new(),
                ended: evaluation.ended,
                timed_out: true,
            });
        }
        tokio::select! {
            _ = wake.changed() => {}
            _ = tokio::time::sleep_until((now + RESCAN).min(deadline)) => {}
        }
    }
}

struct Evaluation {
    matched: Vec<AgentSnapshot>,
    ended: Vec<AgentTarget>,
}

fn evaluate(
    request: &AgentWaitRequest,
    source: &WaitSnapshot,
    first: bool,
) -> Result<Evaluation, PublicError> {
    let mut matched = Vec::new();
    let mut ended = Vec::new();
    for target in &request.targets {
        // A target that is not current was replaced or removed. History keeps
        // the old snapshot, so presence in the projection is not enough.
        if !source.current.contains(target) {
            if first {
                return Err(PublicError::NotFound);
            }
            ended.push(*target);
            continue;
        }
        let Some(snapshot) = source
            .agents
            .iter()
            .find(|agent| agent.cursor().target() == *target)
        else {
            if first {
                return Err(PublicError::NotFound);
            }
            ended.push(*target);
            continue;
        };
        if !newer_than(snapshot, request.after.as_ref()) {
            continue;
        }
        if request.condition.matches(snapshot) {
            matched.push(snapshot.clone());
        }
    }
    Ok(Evaluation { matched, ended })
}

/// With a cursor, only an observation of the same or a replacement execution
/// that advanced past it counts. Without one the current state may match.
fn newer_than(snapshot: &AgentSnapshot, after: Option<&ObservationCursor>) -> bool {
    let Some(after) = after else {
        return true;
    };
    let cursor = snapshot.cursor();
    cursor.target() != after.target()
        || cursor.stream() != after.stream()
        || cursor.sequence() > after.sequence()
}

#[cfg(all(test, feature = "yolo-unit-tests"))]
#[path = "../unit_tests/lib.rs"]
mod unit_tests;
