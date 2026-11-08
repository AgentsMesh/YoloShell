use std::{
    sync::{Arc, Mutex},
    time::Instant,
};

/// Confirmed lease lifetime for one immutable writer identity. Separate from
/// per-attempt RPC timeouts, which are captured once when dispatch starts.
#[derive(Debug, Default)]
pub(crate) struct LeaseDeadline(Mutex<Option<Instant>>);
impl LeaseDeadline {
    pub fn initialize(&self, expires: Instant) {
        self.0.lock().unwrap().get_or_insert(expires);
    }
    pub fn current(&self, fallback: Instant) -> Instant {
        self.0.lock().unwrap().unwrap_or(fallback)
    }
    pub fn renew(&self, expires: Instant, now: Instant) -> bool {
        let mut current = self.0.lock().unwrap();
        match *current {
            Some(previous) if previous <= now => false,
            Some(previous) => {
                *current = Some(previous.max(expires));
                true
            }
            None => {
                *current = Some(expires);
                expires > now
            }
        }
    }
}
pub(crate) type SharedLeaseDeadline = Arc<LeaseDeadline>;
