mod completion;
#[cfg(all(test, not(feature = "yolo-integration-tests")))]
#[path = "../../unit_tests/control/control_recovery_deadlines.rs"]
mod deadline_tests;
mod presentation;
mod probe;
mod reconcile;
mod takeover;

pub(crate) use completion::{failed, granted, not_sent};
pub(crate) use presentation::{badge, detail, label, marker, phase, routine};
pub(crate) use probe::{completed as probe_completed, refresh as refresh_observations};
pub(crate) use reconcile::{acquire, dispatch, reconcile};
pub(crate) use takeover::{observe_reply, takeover};

use std::{
    collections::BTreeMap,
    time::{Instant, SystemTime, UNIX_EPOCH},
};
use yolo_hub_client_channels::RequestScope;
use yolo_hub_identity::{RuntimeTarget, TerminalAddress};
use yolo_hub_terminal_protocol::{LeaseMode, LeaseObservation};
use yolo_tui_terminal_control::{Controller, Intent};

#[derive(Default)]
pub(crate) struct Controls {
    entries: BTreeMap<TerminalAddress, Entry>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum OwnerClass {
    ThisController,
    SameClient,
    Other,
}

struct Entry {
    policy: Controller,
    scope: Option<(RequestScope, LeaseMode)>,
    takeover: Option<(RequestScope, Instant)>,
    probe: Option<probe::Probe>,
    next_probe: Instant,
    observation: Option<yolo_hub_terminal_protocol::LeaseObservationFields>,
}

impl Controls {
    pub fn desire(&mut self, target: RuntimeTarget, now: Instant) {
        self.entry(target).policy.request_write(now);
    }

    fn entry(&mut self, target: RuntimeTarget) -> &mut Entry {
        let entry = self.entries.entry(target.address).or_insert_with(|| Entry {
            policy: Controller::new(target),
            scope: None,
            takeover: None,
            probe: None,
            next_probe: Instant::now(),
            observation: None,
        });
        if entry.policy.target() != target {
            *entry = Entry {
                policy: Controller::new(target),
                scope: None,
                takeover: None,
                probe: None,
                next_probe: Instant::now(),
                observation: None,
            };
        }
        entry
    }

    pub fn adopt(&mut self, target: RuntimeTarget, now: Instant) {
        let entry = self.entry(target);
        entry.policy.adopt(now);
        entry.scope = None;
        entry.takeover = None;
        entry.probe = None;
        entry.observation = None;
    }

    pub fn observe(&mut self, target: RuntimeTarget) {
        let entry = self.entry(target);
        entry.policy.observe();
        entry.scope = None;
        entry.takeover = None;
        entry.probe = None;
        entry.next_probe = Instant::now();
        entry.observation = None;
    }

    /// The controller is actively pursuing write authority for this exact
    /// target. Contention and denials are not an admission window: the user
    /// must resolve those explicitly.
    pub fn acquiring(&self, target: RuntimeTarget, now: Instant) -> bool {
        self.entries.get(&target.address).is_some_and(|entry| {
            entry.policy.target() == target
                && matches!(
                    entry.policy.phase(now, false),
                    yolo_tui_terminal_control::Presentation::Acquiring
                        | yolo_tui_terminal_control::Presentation::Recovering
                )
        })
    }

    pub fn permits(&self, target: RuntimeTarget) -> bool {
        self.entries.get(&target.address).is_none_or(|entry| {
            entry.policy.target() == target && entry.policy.intent() == Intent::Write
        })
    }

    pub fn lost(&mut self, address: TerminalAddress, now: Instant) {
        if let Some(entry) = self.entries.get_mut(&address) {
            entry.policy.lost(now);
        }
    }

    pub fn retire(&mut self, address: TerminalAddress) {
        if let Some(entry) = self.entries.get_mut(&address) {
            entry.policy.lost(Instant::now());
            entry.scope = None;
            entry.takeover = None;
            entry.probe = None;
            entry.observation = None;
        }
    }

    pub fn record_observation(&mut self, target: RuntimeTarget, lease: Option<&LeaseObservation>) {
        let entry = self.entry(target);
        entry.observation = lease
            .filter(|value| value.fields().target == target)
            .map(|value| value.fields().clone());
    }

    pub fn clear_observation(&mut self, address: TerminalAddress) {
        if let Some(entry) = self.entries.get_mut(&address) {
            entry.observation = None;
        }
    }

    pub fn observed_holder(&self, address: TerminalAddress) -> Option<&str> {
        self.entries
            .get(&address)
            .and_then(|entry| entry.observation.as_ref())
            .filter(|observation| {
                SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .map(|now| observation.expires_at_ms > now.as_millis() as i64)
                    .unwrap_or(false)
            })
            .map(|observation| observation.holder_id.as_str())
    }

    pub fn observed_owner(
        &self,
        address: TerminalAddress,
        principal: yolo_hub_terminal_protocol::WriterPrincipal,
    ) -> Option<OwnerClass> {
        let observation = self
            .entries
            .get(&address)
            .and_then(|entry| entry.observation.as_ref())
            .filter(|observation| {
                SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .map(|now| observation.expires_at_ms > now.as_millis() as i64)
                    .unwrap_or(false)
            })?;
        let same_client = observation.holder_id == principal.client_id.to_string();
        if same_client
            && observation.controller_instance_id.as_deref()
                == Some(principal.controller_instance_id.to_string().as_str())
        {
            Some(OwnerClass::ThisController)
        } else if same_client {
            Some(OwnerClass::SameClient)
        } else {
            Some(OwnerClass::Other)
        }
    }

    pub fn disconnected(&mut self, now: Instant) {
        for entry in self.entries.values_mut() {
            entry.policy.disconnected(now);
            entry.scope = None;
            entry.takeover = None;
            entry.probe = None;
            entry.next_probe = now;
            entry.observation = None;
        }
    }
}
