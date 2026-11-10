use super::*;
use anyhow::{Result, anyhow};
use std::sync::Arc;
use std::time::{Duration, Instant};

#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-integration-tests"))]
#[path = "../../integration_tests/process_file_lock/process_file_lock_turn_waiter_tests.rs"]
mod tests;

const TURN_DEADLINE_OVERFLOW: &str = "process lock turn deadline overflow";

pub(super) struct TurnWaiter {
    deadline: TurnDeadline,
    identity: Arc<()>,
    idle_timeout: Option<Duration>,
}

#[derive(Clone, Copy)]
pub(super) enum TurnDeadline {
    Active(Instant),
    Expired,
    Overflow,
}

impl TurnWaiter {
    pub(super) fn idle(idle_timeout: Duration) -> Result<Self> {
        let deadline = deadline_after(Instant::now(), idle_timeout)?;
        Ok(Self {
            deadline: TurnDeadline::Active(deadline),
            identity: Arc::new(()),
            idle_timeout: Some(idle_timeout),
        })
    }

    pub(super) fn absolute(deadline: Instant) -> Self {
        Self {
            deadline: TurnDeadline::Active(deadline),
            identity: Arc::new(()),
            idle_timeout: None,
        }
    }

    pub(super) fn advance_at(&mut self, occurred_at: Instant) {
        let (TurnDeadline::Active(deadline), Some(idle_timeout)) =
            (self.deadline, self.idle_timeout)
        else {
            return;
        };
        if occurred_at >= deadline {
            self.deadline = TurnDeadline::Expired;
            return;
        }
        self.deadline = match occurred_at.checked_add(idle_timeout) {
            Some(deadline) => TurnDeadline::Active(deadline),
            None => TurnDeadline::Overflow,
        };
    }

    pub(super) fn identity(&self) -> Arc<()> {
        Arc::clone(&self.identity)
    }

    pub(super) fn allows_initial_grant(&self) -> bool {
        self.idle_timeout.is_some()
    }

    #[cfg(test)]
    pub(super) fn arm_renewal_overflow_for_test(&mut self) {
        self.idle_timeout = Some(Duration::MAX);
    }

    #[cfg(test)]
    pub(super) fn expire_for_test(&mut self) {
        self.deadline = TurnDeadline::Expired;
    }

    pub(super) fn matches(&self, identity: &Arc<()>) -> bool {
        Arc::ptr_eq(&self.identity, identity)
    }
}

impl TurnDeadline {
    pub(super) fn remaining(self, now: Instant) -> Result<Option<Duration>> {
        match self {
            Self::Active(deadline) => Ok(deadline.checked_duration_since(now)),
            Self::Expired => Ok(None),
            Self::Overflow => Err(anyhow!(TURN_DEADLINE_OVERFLOW)),
        }
    }
}

pub(super) fn observe(
    registry: &Registry,
    key: ObjectIdentity,
    identity: &Arc<()>,
) -> (wait_signal::ProcessWaitToken, TurnDeadline, bool) {
    let state = registry.turns.get(&key).expect(TURN_STATE_MISSING);
    let queued = state
        .waiters
        .iter()
        .find(|queued| queued.matches(identity))
        .expect(TURN_STATE_MISSING);
    (
        state.signal.token(),
        queued.deadline,
        can_grant(registry, key, identity),
    )
}

pub(super) fn can_grant(registry: &Registry, key: ObjectIdentity, identity: &Arc<()>) -> bool {
    let state = registry.turns.get(&key).expect(TURN_STATE_MISSING);
    !state.owner
        && state
            .waiters
            .front()
            .is_some_and(|front| front.matches(identity))
}

pub(super) fn grant(registry: &mut Registry, key: ObjectIdentity) {
    let state = registry.turns.get_mut(&key).expect(TURN_STATE_MISSING);
    state.waiters.pop_front().expect(TURN_STATE_MISSING);
    state.owner = true;
}

fn deadline_after(start: Instant, timeout: Duration) -> Result<Instant> {
    start
        .checked_add(timeout)
        .ok_or_else(|| anyhow!(TURN_DEADLINE_OVERFLOW))
}
