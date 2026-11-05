use crate::{ClaudeLifecycleEvent, ClaudeNativeRequest, ClaudeNativeScope};
use anyhow::{Context, Result, ensure};
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::path::Path;

#[path = "native_outbox_storage.rs"]
mod storage;

const INVALID: &str = "Claude native hook outbox is invalid or changed";
const CAPACITY: &str = "Claude native hook outbox capacity is exhausted";
const INCREMENT: u64 = 1;

#[derive(Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Outbox {
    queues: Vec<Queue>,
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Queue {
    scope: ClaudeNativeScope,
    acknowledged: u64,
    events: VecDeque<ClaudeNativeRequest>,
}

pub fn enqueue(path: &Path, scope: ClaudeNativeScope, event: ClaudeLifecycleEvent) -> Result<()> {
    ensure!(scope.is_valid() && !event.event_id().is_nil(), INVALID);
    storage::update(path, |state| {
        state.queues.retain(|queue| {
            queue.scope == scope
                || !queue.events.is_empty()
                || crate::process::process_identity_relation(
                    queue.scope.agent.pid,
                    queue.scope.agent.start_time,
                ) != crate::process::ProcessIdentityRelation::Gone
        });
        let index = match state.queues.iter().position(|queue| queue.scope == scope) {
            Some(index) => index,
            None => {
                ensure!(
                    state.queues.len() < crate::MAX_CLAUDE_OUTBOX_INVOCATIONS,
                    CAPACITY
                );
                state.queues.push(Queue {
                    scope,
                    acknowledged: u64::default(),
                    events: VecDeque::new(),
                });
                state.queues.len() - INCREMENT as usize
            }
        };
        let queue = &mut state.queues[index];
        if let Some(existing) = queue
            .events
            .iter()
            .find(|item| item.event.event_id() == event.event_id())
        {
            ensure!(existing.event == event, INVALID);
            return Ok(());
        }
        ensure!(
            queue.events.len() < crate::MAX_CLAUDE_OUTBOX_EVENTS,
            CAPACITY
        );
        let sequence = queue
            .events
            .back()
            .map_or(queue.acknowledged, |item| item.event_sequence)
            .checked_add(INCREMENT)
            .context(CAPACITY)?;
        queue.events.push_back(ClaudeNativeRequest {
            schema_version: crate::CLAUDE_NATIVE_SCHEMA_VERSION,
            scope,
            event_sequence: sequence,
            event,
        });
        Ok(())
    })
}

pub fn next(path: &Path, scope: ClaudeNativeScope) -> Result<Option<ClaudeNativeRequest>> {
    storage::read(path, |state| {
        Ok(state
            .queues
            .iter()
            .find(|queue| queue.scope == scope)
            .and_then(|queue| queue.events.front())
            .cloned())
    })
}

pub fn acknowledge(path: &Path, request: &ClaudeNativeRequest) -> Result<()> {
    storage::update(path, |state| {
        let queue = state
            .queues
            .iter_mut()
            .find(|queue| queue.scope == request.scope)
            .context(INVALID)?;
        if request.event_sequence <= queue.acknowledged {
            return Ok(());
        }
        ensure!(queue.events.front() == Some(request), INVALID);
        queue.events.pop_front();
        queue.acknowledged = request.event_sequence;
        Ok(())
    })
}

impl Outbox {
    /// Requests persisted by the pre-interaction bridge are still deliverable;
    /// normalize their schema before the strict validation runs.
    fn migrate(&mut self) {
        for queue in &mut self.queues {
            for request in &mut queue.events {
                if request.schema_version == crate::PREVIOUS_CLAUDE_NATIVE_SCHEMA_VERSION {
                    request.schema_version = crate::CLAUDE_NATIVE_SCHEMA_VERSION;
                }
            }
        }
    }

    fn validate(&self) -> Result<()> {
        ensure!(
            self.queues.len() <= crate::MAX_CLAUDE_OUTBOX_INVOCATIONS,
            CAPACITY
        );
        for (index, queue) in self.queues.iter().enumerate() {
            ensure!(
                queue.scope.is_valid()
                    && !self.queues[..index]
                        .iter()
                        .any(|prior| prior.scope == queue.scope)
                    && queue.events.len() <= crate::MAX_CLAUDE_OUTBOX_EVENTS,
                INVALID
            );
            let mut previous = queue.acknowledged;
            let mut ids = std::collections::HashSet::new();
            for request in &queue.events {
                ensure!(
                    request.is_valid()
                        && request.scope == queue.scope
                        && previous.checked_add(INCREMENT) == Some(request.event_sequence)
                        && ids.insert(request.event.event_id()),
                    INVALID
                );
                previous = request.event_sequence;
            }
        }
        Ok(())
    }
}

#[cfg(all(test, feature = "yolo-integration-tests"))]
#[path = "../integration_tests/native_outbox_tests.rs"]
mod integration_tests;
#[cfg(all(test, feature = "yolo-unit-tests"))]
#[path = "../unit_tests/native_outbox_tests.rs"]
mod tests;
