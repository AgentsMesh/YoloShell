const OWNER_CONTAINMENT_PAYLOAD_PANIC: &str = "injected PTY owner payload Drop panic";
const OWNER_CONTAINMENT_SPAN_ID: u64 = 1;

struct OwnerContainmentPayload(Arc<AtomicBool>);

impl Drop for OwnerContainmentPayload {
    fn drop(&mut self) {
        self.0.store(true, Ordering::SeqCst);
        panic!("{OWNER_CONTAINMENT_PAYLOAD_PANIC}");
    }
}

struct OwnerContainmentSubscriber {
    events: Arc<std::sync::atomic::AtomicUsize>,
    payload_dropped: Arc<AtomicBool>,
}

impl tracing::Subscriber for OwnerContainmentSubscriber {
    fn enabled(&self, _metadata: &tracing::Metadata<'_>) -> bool {
        true
    }

    fn new_span(&self, _attributes: &tracing::span::Attributes<'_>) -> tracing::span::Id {
        tracing::span::Id::from_u64(OWNER_CONTAINMENT_SPAN_ID)
    }

    fn record(&self, _span: &tracing::span::Id, _values: &tracing::span::Record<'_>) {}
    fn record_follows_from(&self, _span: &tracing::span::Id, _follows: &tracing::span::Id) {}

    fn event(&self, _event: &tracing::Event<'_>) {
        self.events.fetch_add(1, Ordering::SeqCst);
        std::panic::panic_any(OwnerContainmentPayload(Arc::clone(&self.payload_dropped)));
    }

    fn enter(&self, _span: &tracing::span::Id) {}
    fn exit(&self, _span: &tracing::span::Id) {}
}
