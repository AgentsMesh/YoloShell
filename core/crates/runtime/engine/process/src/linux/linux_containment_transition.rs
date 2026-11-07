use super::constants::*;
use super::io::{read_control, write_control};
use super::membership::read_member_pids;
use super::{LinuxContainmentError, LinuxContainmentLease};
use crate::process::LiveProcessIdentity;
use std::collections::HashSet;
use std::time::Instant;

const EVENTS_MALFORMED_DETAIL: &str = "cgroup events are malformed";
const FREEZE_TIMEOUT_DETAIL: &str = "session cgroup did not freeze before its deadline";
const MEMBERSHIP_CHANGED_DETAIL: &str = "frozen cgroup membership changed while observed";
const EVENT_FALSE: &str = "0";
const EVENT_TRUE: &str = "1";

#[path = "linux_containment_observation.rs"]
mod observation;

#[derive(Clone, Copy)]
struct Events {
    populated: bool,
    frozen: bool,
}

impl LinuxContainmentLease {
    pub(crate) fn poll_frozen_members(
        &mut self,
        request_freeze: bool,
    ) -> Result<Option<HashSet<LiveProcessIdentity>>, LinuxContainmentError> {
        let freeze_armed = request_freeze || self.freeze_requested_at.is_some();
        let observed = read_events(&self.leaf);
        if freeze_armed {
            write_control(&self.leaf, CGROUP_FREEZE_FILE, FREEZE_VALUE)?;
            if self.freeze_requested_at.is_none() {
                self.freeze_requested_at = Some(Instant::now());
            }
        }
        let observed = observed?;
        let before = read_events(&self.leaf)?;
        if !before.frozen || (freeze_armed && !observed.frozen) {
            return pending_before_deadline(self.freeze_requested_at, Instant::now());
        }
        let first = read_member_pids(&self.leaf)?;
        let identities = observation::capture_members(&first)?;
        let second = read_member_pids(&self.leaf)?;
        let after = read_events(&self.leaf)?;
        if freeze_armed && !after.frozen {
            write_control(&self.leaf, CGROUP_FREEZE_FILE, FREEZE_VALUE)?;
        }
        if !after.frozen {
            return pending_before_deadline(self.freeze_requested_at, Instant::now());
        }
        if identities.is_none() || validate_observation(before, &first, &second, after).is_err() {
            return observation::pending_membership(
                &mut self.membership_unstable_since,
                Instant::now(),
            );
        }
        self.membership_unstable_since = None;
        Ok(identities)
    }
}

fn pending_before_deadline(
    started: Option<Instant>,
    now: Instant,
) -> Result<Option<HashSet<LiveProcessIdentity>>, LinuxContainmentError> {
    if started.is_some_and(|started| now.saturating_duration_since(started) >= FREEZE_TIMEOUT) {
        Err(LinuxContainmentError::Unavailable(FREEZE_TIMEOUT_DETAIL))
    } else {
        Ok(None)
    }
}

fn validate_observation(
    before: Events,
    first: &std::collections::BTreeSet<u32>,
    second: &std::collections::BTreeSet<u32>,
    after: Events,
) -> Result<(), LinuxContainmentError> {
    if first != second
        || !before.frozen
        || !after.frozen
        || before.populated != after.populated
        || after.populated != !second.is_empty()
    {
        return Err(LinuxContainmentError::Identity(MEMBERSHIP_CHANGED_DETAIL));
    }
    Ok(())
}

fn read_events(directory: &std::fs::File) -> Result<Events, LinuxContainmentError> {
    parse_events(&read_control(directory, CGROUP_EVENTS_FILE)?)
}

fn parse_events(content: &str) -> Result<Events, LinuxContainmentError> {
    let mut populated = None;
    let mut frozen = None;
    for line in content.lines() {
        let mut fields = line.split_ascii_whitespace();
        let key = fields
            .next()
            .ok_or(LinuxContainmentError::Identity(EVENTS_MALFORMED_DETAIL))?;
        let value = fields
            .next()
            .and_then(parse_bool)
            .ok_or(LinuxContainmentError::Identity(EVENTS_MALFORMED_DETAIL))?;
        if fields.next().is_some() {
            return Err(LinuxContainmentError::Identity(EVENTS_MALFORMED_DETAIL));
        }
        match key {
            REQUIRED_EVENT_POPULATED if populated.replace(value).is_some() => {
                return Err(LinuxContainmentError::Identity(EVENTS_MALFORMED_DETAIL));
            }
            REQUIRED_EVENT_FROZEN if frozen.replace(value).is_some() => {
                return Err(LinuxContainmentError::Identity(EVENTS_MALFORMED_DETAIL));
            }
            _ => {}
        }
    }
    Ok(Events {
        populated: populated.ok_or(LinuxContainmentError::Identity(EVENTS_MALFORMED_DETAIL))?,
        frozen: frozen.ok_or(LinuxContainmentError::Identity(EVENTS_MALFORMED_DETAIL))?,
    })
}

fn parse_bool(value: &str) -> Option<bool> {
    match value {
        EVENT_FALSE => Some(false),
        EVENT_TRUE => Some(true),
        _ => None,
    }
}

#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-integration-tests"))]
#[path = "../../integration_tests/linux/linux_containment_transition_tests.rs"]
mod tests;
