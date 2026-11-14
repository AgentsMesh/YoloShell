use crate::field;

#[doc(hidden)]
pub const REDACTED_ENVELOPE_PAYLOAD: &str = "[REDACTED]";

impl std::fmt::Debug for RequestEnvelope {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("RequestEnvelope")
            .field(field::VERSION, &self.version)
            .field(field::KIND, &self.kind)
            .field(field::REQUEST_ID, &self.request_id)
            .field(field::METHOD, &self.method)
            .field(field::PAYLOAD, &REDACTED_ENVELOPE_PAYLOAD)
            .finish()
    }
}

impl std::fmt::Debug for ResponseEnvelope {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("ResponseEnvelope")
            .field(field::VERSION, &self.version)
            .field(field::KIND, &self.kind)
            .field(field::REQUEST_ID, &self.request_id)
            .field(field::METHOD, &self.method)
            .field(field::OK, &self.ok)
            .field(field::PAYLOAD, &REDACTED_ENVELOPE_PAYLOAD)
            .field(field::ERROR, &self.error)
            .finish()
    }
}

impl std::fmt::Debug for EventEnvelope {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("EventEnvelope")
            .field(field::VERSION, &self.version)
            .field(field::KIND, &self.kind)
            .field(field::EVENT, &self.event)
            .field(field::TERMINAL_ID, &self.terminal_id)
            .field(field::RUNTIME_EPOCH, &self.runtime_epoch)
            .field(field::PAYLOAD, &REDACTED_ENVELOPE_PAYLOAD)
            .finish()
    }
}
