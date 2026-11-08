use std::sync::{Mutex, MutexGuard};

static SUBSCRIBER_TEST_GATE: Mutex<()> = Mutex::new(());

pub(crate) struct SubscriberTestGuard {
    _cache_sentinel: tracing::Dispatch,
    _gate: MutexGuard<'static, ()>,
}

pub(crate) fn serialize_subscriber_tests() -> SubscriberTestGuard {
    let gate = SUBSCRIBER_TEST_GATE
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let cache_sentinel = tracing::Dispatch::new(tracing::subscriber::NoSubscriber::default());
    SubscriberTestGuard {
        _cache_sentinel: cache_sentinel,
        _gate: gate,
    }
}
