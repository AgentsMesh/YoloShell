use crate::{Value, api::*};
use std::cell::{Cell, RefCell};
use std::sync::Arc;
use tokio::sync::{OwnedSemaphorePermit, watch};
use yolo_capnp_schema::rpc_capnp::events;

struct Events {
    source: RefCell<Option<Box<dyn EventSource>>>,
    cancelled: Cell<bool>,
    stop: watch::Sender<bool>,
    _permit: OwnedSemaphorePermit,
    domain: tokio::runtime::Handle,
    scope: Arc<crate::domain::Scope>,
}
pub(crate) fn client(
    source: Box<dyn EventSource>,
    permit: OwnedSemaphorePermit,
    domain: tokio::runtime::Handle,
    scope: Arc<crate::domain::Scope>,
) -> events::Client {
    let (stop, _) = watch::channel(false);
    capnp_rpc::new_client(Events {
        source: RefCell::new(Some(source)),
        cancelled: Cell::new(false),
        stop,
        _permit: permit,
        domain,
        scope,
    })
}
impl events::Server for Events {
    async fn next(
        self: capnp::capability::Rc<Self>,
        _: events::NextParams,
        mut results: events::NextResults,
    ) -> Result<(), capnp::Error> {
        if self.cancelled.get() {
            return Err(rpc_error(Error::Closed));
        }
        let mut source = self
            .source
            .borrow_mut()
            .take()
            .ok_or_else(|| rpc_error(Error::Closed))?;
        let mut stop = self.stop.subscribe();
        let mut task = self
            .scope
            .spawn(&self.domain, async move {
                let result = source.next().await;
                (result, source)
            })
            .map_err(rpc_error)?;
        let (value, source) = tokio::select! {
            _ = stop.changed() => return Err(rpc_error(Error::Closed)),
            value = &mut task.handle => value.map_err(|error| capnp::Error::failed(error.to_string()))?,
        };
        let value: Value = value.map_err(rpc_error)?;
        if self.cancelled.get() {
            return Err(rpc_error(Error::Closed));
        }
        yolo_capnp_codec::write_value(results.get().init_event(), &value)
            .map_err(|error| rpc_error(error.into()))?;
        self.source.replace(Some(source));
        Ok(())
    }
    async fn cancel(
        self: capnp::capability::Rc<Self>,
        _: events::CancelParams,
        _: events::CancelResults,
    ) -> Result<(), capnp::Error> {
        self.cancelled.set(true);
        self.stop.send_replace(true);
        self.source.borrow_mut().take();
        Ok(())
    }
}
