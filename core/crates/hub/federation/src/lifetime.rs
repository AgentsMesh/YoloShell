use crate::{Federation, constants::CLOSED};
use anyhow::{Context, Result, ensure};
use std::sync::{Arc, Mutex};
use tokio::{sync::Notify, time::Instant};

const EXHAUSTED: &str = "Hub generation activity count exhausted";
const NOT_RETIRED: &str = "Hub generation drain requires closed admission";
const DRAIN_TIMEOUT: &str = "Hub generation still owns asynchronous protocol work";

#[derive(Default)]
struct State {
    retired: bool,
    active: usize,
}

/// Counts the owners that may outlive their network service task.
#[derive(Default)]
pub(crate) struct Lifetime {
    state: Mutex<State>,
    changed: Notify,
}

impl Lifetime {
    fn acquire(self: &Arc<Self>) -> Result<Ticket> {
        let mut state = self.state.lock().unwrap();
        ensure!(!state.retired, CLOSED);
        state.active = state.active.checked_add(1).context(EXHAUSTED)?;
        Ok(Ticket(self.clone()))
    }

    pub fn retire(&self) {
        self.state.lock().unwrap().retired = true;
        self.changed.notify_waiters();
    }

    pub async fn drained(&self, deadline: Instant) -> Result<()> {
        loop {
            let changed = self.changed.notified();
            tokio::pin!(changed);
            changed.as_mut().enable();
            {
                let state = self.state.lock().unwrap();
                ensure!(state.retired, NOT_RETIRED);
                if state.active == 0 {
                    return Ok(());
                }
            }
            tokio::time::timeout_at(deadline, changed)
                .await
                .context(DRAIN_TIMEOUT)?;
        }
    }
}

struct Ticket(Arc<Lifetime>);

impl Drop for Ticket {
    fn drop(&mut self) {
        let mut state = self.0.state.lock().unwrap();
        state.active -= 1;
        if state.active == 0 {
            self.0.changed.notify_waiters();
        }
    }
}

/// Field order releases the generation before publishing activity completion.
pub(crate) struct OwnedGeneration {
    node: Arc<Federation>,
    _ticket: Ticket,
}

impl OwnedGeneration {
    pub fn new(node: Arc<Federation>) -> Result<Self> {
        node.require_open()?;
        let ticket = node.lifetime.acquire()?;
        Ok(Self {
            node,
            _ticket: ticket,
        })
    }

    pub fn node(&self) -> &Arc<Federation> {
        &self.node
    }
}

#[cfg(all(test, feature = "yolo-unit-tests"))]
#[path = "../unit_tests/lifetime.rs"]
mod unit_tests;
