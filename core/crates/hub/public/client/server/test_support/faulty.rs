use super::*;
use std::sync::atomic::{AtomicBool, Ordering};
use yolo_hub_dispatch::{Dispatcher, OperationFuture};
use yolo_hub_rpc_protocol::{Command, Request};

pub struct FaultyBackend {
    pub inner: Arc<Backend>,
    pub panic_once: AtomicBool,
}
impl Dispatcher for FaultyBackend {
    fn hub_id(&self) -> HubId {
        self.inner.hub
    }
    fn dispatch(&self, request: Request) -> OperationFuture<'_> {
        Box::pin(async move {
            if matches!(
                request.fields().command,
                Command::List {
                    include_archived: true,
                    ..
                }
            ) && self.panic_once.swap(false, Ordering::SeqCst)
            {
                panic!("{EXPECTED_DISPATCH_PANIC}");
            }
            self.inner.dispatch(request).await
        })
    }
}
