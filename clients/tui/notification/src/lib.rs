//! Terminal notification cursors: bell and status-bar messages derived from
//! catalogue presentation sequences.

mod constants;

use std::{collections::BTreeMap, time::Instant};
use yolo_hub_identity::{RuntimeEpoch, RuntimeTarget, TerminalAddress};
use yolo_hub_terminal_protocol::{RuntimeStatus, TerminalRecoveryHint, TerminalSummary};

#[derive(Default)]
pub struct Notifications {
    cursors: BTreeMap<TerminalAddress, (RuntimeEpoch, u64)>,
    latest: Option<(RuntimeTarget, Instant, String)>,
    ring: bool,
}

impl Notifications {
    /// A catalogue baseline or reconnect is observation, not a new notification.
    pub fn observe(
        &mut self,
        terminals: &[TerminalSummary],
        current: bool,
        attention: &mut std::collections::BTreeSet<TerminalAddress>,
        now: Instant,
    ) {
        self.cursors
            .retain(|address, _| terminals.iter().any(|t| t.fields().address == *address));
        attention.retain(|address| self.cursors.contains_key(address));
        if self
            .latest
            .as_ref()
            .is_some_and(|(target, _, _)| !self.cursors.contains_key(&target.address))
        {
            self.latest = None;
            self.ring = false;
        }
        for terminal in terminals {
            let fields = terminal.fields();
            let Some(runtime) = fields.runtime else {
                continue;
            };
            let presentation = &fields.presentation;
            let next = (
                runtime.target.runtime_epoch,
                presentation.notification_sequence,
            );
            let previous = self.cursors.get(&fields.address).copied();
            self.cursors.insert(
                fields.address,
                (
                    next.0,
                    previous
                        .filter(|(epoch, _)| *epoch == next.0)
                        .map_or(next.1, |(_, sequence)| sequence.max(next.1)),
                ),
            );
            if previous.is_some_and(|(epoch, _)| epoch != next.0) {
                attention.remove(&fields.address);
                if self
                    .latest
                    .as_ref()
                    .is_some_and(|(target, _, _)| target.address == fields.address)
                {
                    self.latest = None;
                }
            }
            if !current
                || fields.archived
                || fields.recovery == Some(TerminalRecoveryHint::Unavailable)
                || matches!(
                    runtime.status,
                    RuntimeStatus::Unreachable | RuntimeStatus::Lost
                )
                || !previous.is_some_and(|(epoch, sequence)| epoch == next.0 && sequence < next.1)
            {
                continue;
            }
            let Some(notification) = &presentation.notification else {
                continue;
            };
            attention.insert(fields.address);
            self.ring = true;
            let body = [&notification.title, &notification.body]
                .into_iter()
                .filter(|text| !text.is_empty())
                .cloned()
                .collect::<Vec<_>>()
                .join(constants::SEPARATOR);
            let body = if body.is_empty() {
                constants::ALERT_LABEL.into()
            } else {
                body
            };
            self.latest = Some((
                runtime.target,
                now + constants::DISPLAY_DURATION,
                format!("{}{}{body}", terminal.display_title(), constants::SEPARATOR),
            ));
        }
    }

    pub fn message(&self, now: Instant) -> Option<&str> {
        self.latest
            .as_ref()
            .filter(|(_, expires, _)| now < *expires)
            .map(|(_, _, text)| text.as_str())
    }

    pub fn take_bell(&mut self) -> Option<&'static [u8]> {
        std::mem::take(&mut self.ring).then_some(constants::BELL)
    }
}
