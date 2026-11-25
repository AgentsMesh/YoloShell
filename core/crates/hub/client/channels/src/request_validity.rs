use std::{
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::Instant,
};

/// Local queue lifetime only. This never grants remote writer authority.
#[derive(Debug, Default)]
pub struct RequestScope {
    retired: Arc<AtomicBool>,
    ordered: bool,
    lease: Option<crate::lease_deadline::SharedLeaseDeadline>,
    stream_failed: Option<Arc<AtomicBool>>,
}

impl RequestScope {
    /// Input requests in this scope depend on every preceding acknowledgement.
    /// A rejected or unconfirmed request cancels all later members before dispatch.
    pub fn ordered_inputs() -> Self {
        Self {
            retired: Arc::default(),
            ordered: true,
            lease: None,
            stream_failed: None,
        }
    }
    pub fn leased_inputs(expires: Instant) -> Self {
        let lease = Arc::new(crate::lease_deadline::LeaseDeadline::default());
        lease.initialize(expires);
        Self {
            lease: Some(lease),
            retired: Arc::default(),
            ordered: true,
            stream_failed: None,
        }
    }

    pub(crate) fn input_view(stream: &crate::InputStream) -> Self {
        Self {
            retired: Arc::default(),
            ordered: true,
            lease: Some(Arc::clone(&stream.lease)),
            stream_failed: Some(Arc::clone(&stream.failed)),
        }
    }

    pub fn validity_until(&self, expires: Instant) -> RequestValidity {
        if let Some(lease) = &self.lease {
            lease.initialize(expires);
        }
        RequestValidity {
            retired: Arc::clone(&self.retired),
            expires,
            ordered: self.ordered,
            lease: self.lease.clone(),
            stream_failed: self.stream_failed.clone(),
        }
    }
    /// Only a confirmed renewal of this scope's exact writer may call this.
    /// Expired or rejected authority is never revived by a late renewal.
    pub fn renew_authority(&self, expires: Instant) -> bool {
        if self.retired.load(Ordering::Acquire)
            || self
                .stream_failed
                .as_ref()
                .is_some_and(|failed| failed.load(Ordering::Acquire))
        {
            return false;
        }
        let renewed = self
            .lease
            .as_ref()
            .is_some_and(|lease| lease.renew(expires, Instant::now()));
        if !renewed {
            self.retire();
        }
        renewed
    }
    pub fn retire(&self) {
        self.retired.store(true, Ordering::Release);
    }
}
impl Drop for RequestScope {
    fn drop(&mut self) {
        self.retire();
    }
}

#[derive(Clone, Debug)]
pub struct RequestValidity {
    retired: Arc<AtomicBool>,
    expires: Instant,
    ordered: bool,
    lease: Option<crate::lease_deadline::SharedLeaseDeadline>,
    stream_failed: Option<Arc<AtomicBool>>,
}
impl RequestValidity {
    pub fn is_current(&self) -> bool {
        !self.retired.load(Ordering::Acquire)
            && !self
                .stream_failed
                .as_ref()
                .is_some_and(|failed| failed.load(Ordering::Acquire))
            && Instant::now() < self.admission_deadline()
    }
    pub fn deadline(&self) -> Instant {
        self.expires
    }
    pub fn admission_deadline(&self) -> Instant {
        self.lease
            .as_ref()
            .map_or(self.expires, |lease| lease.current(self.expires))
    }

    /// Cancel dependent input, without changing ordinary request-scope semantics.
    /// Call before publishing failure or dispatching the next ordered request.
    pub fn reject(&self) {
        if self.ordered {
            self.retired.store(true, Ordering::Release);
        }
    }

    /// Call only for the exact request whose transport operation was attempted.
    /// Navigation/admission cancellation must use reject(), leaving other views.
    pub fn reject_dispatched_input(&self) {
        self.reject();
        if let Some(failed) = &self.stream_failed {
            failed.store(true, Ordering::Release);
        }
    }

    /// Correlate local attempts even after expiry or retirement. Shared scope
    /// identity is not evidence of current validity or remote authority.
    pub fn same_scope(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.retired, &other.retired)
    }
}
