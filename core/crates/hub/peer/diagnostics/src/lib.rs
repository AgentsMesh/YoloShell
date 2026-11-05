use std::collections::BTreeSet;
use std::sync::{Mutex, MutexGuard};
use std::time::{Duration, Instant};
use yolo_hub_identity::HubId;

pub const PEER_ATTEMPT_FAILED: &str = "Hub peer connection attempt failed";
pub const PEER_RECOVERED: &str = "Hub peer connection recovered";
pub const INBOUND_PEER_REJECTED: &str = "Hub rejected an inbound peer connection";
pub const INBOUND_REJECTION_INTERVAL: Duration = Duration::from_secs(30);
pub const EXPORT_DEPTH_EXHAUSTED: &str =
    "Hub export depth exhausted; origin omitted from peer directory";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PeerAttempt {
    Dial,
    Handshake,
    Admission,
}

impl PeerAttempt {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Dial => "dial",
            Self::Handshake => "handshake",
            Self::Admission => "admission",
        }
    }
}

const FAILURE_LOG_EVERY: u64 = 30;

/// Counts consecutive outbound failures per configured peer. A repeated stage
/// is reported only periodically, so a long outage cannot fill the journal.
#[derive(Default)]
pub struct OutboundFailures {
    stage: Option<PeerAttempt>,
    attempts: u64,
}

impl OutboundFailures {
    /// Returns the attempt count when this failure should be logged.
    pub fn fail(&mut self, stage: PeerAttempt) -> Option<u64> {
        self.attempts = self.attempts.saturating_add(1);
        let changed = self.stage != Some(stage);
        self.stage = Some(stage);
        (changed || self.attempts % FAILURE_LOG_EVERY == 0).then_some(self.attempts)
    }

    /// Returns the consecutive failure count when this success ends a streak.
    pub fn succeed(&mut self) -> Option<u64> {
        let attempts = std::mem::take(&mut self.attempts);
        self.stage.take().map(|_| attempts)
    }
}

/// Bounds inbound rejection logging across all handshake tasks. The first
/// rejection is logged immediately; later ones are counted for the next window.
#[derive(Default)]
pub struct InboundRejections {
    window_start: Option<Instant>,
    suppressed: u64,
}

impl InboundRejections {
    /// Returns the suppressed count when a rejection should be logged.
    pub fn reject(&mut self, now: Instant) -> Option<u64> {
        match self.window_start {
            Some(start) if now.duration_since(start) < INBOUND_REJECTION_INTERVAL => {
                self.suppressed = self.suppressed.saturating_add(1);
                None
            }
            _ => {
                self.window_start = Some(now);
                Some(std::mem::take(&mut self.suppressed))
            }
        }
    }
}

type Denial = (HubId, HubId, u64);

/// Reports each denied (peer, origin, policy epoch) transition once. The key
/// is removed when a refresh exports that origin again, so a recurrence is
/// still visible.
#[derive(Default)]
pub struct ExportDepthDenials {
    reported: Mutex<BTreeSet<Denial>>,
}

impl ExportDepthDenials {
    /// Returns true when this denial has not already been reported.
    pub fn denied(&self, peer: HubId, origin: HubId, policy_epoch: u64) -> bool {
        self.lock().insert((peer, origin, policy_epoch))
    }

    pub fn exported(&self, peer: HubId, origin: HubId, policy_epoch: u64) {
        self.lock().remove(&(peer, origin, policy_epoch));
    }

    fn lock(&self) -> MutexGuard<'_, BTreeSet<Denial>> {
        self.reported
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }
}

