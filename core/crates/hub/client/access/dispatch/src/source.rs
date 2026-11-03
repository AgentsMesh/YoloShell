use yolo_hub_client_access_registry::ClientAdmission;
use yolo_hub_dispatch::{EventFuture, TerminalSource};
use yolo_hub_terminal_protocol::AttachResult;

pub(crate) struct ScopedSource {
    pub inner: Box<dyn TerminalSource>,
    pub admission: ClientAdmission,
}

impl TerminalSource for ScopedSource {
    fn initial(&self) -> &AttachResult {
        self.inner.initial()
    }
    fn next_event(&mut self) -> EventFuture<'_> {
        Box::pin(crate::while_authorized(
            &self.admission,
            self.inner.next_event(),
        ))
    }
}
