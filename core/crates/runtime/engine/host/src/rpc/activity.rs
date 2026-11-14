use super::super::{HOST_REQUEST_TIMEOUT, HOST_REQUEST_TIMEOUT_CONTEXT, Host};
use std::sync::Arc;
use tokio::sync::watch;
use yolo_capnp_rpc::{Confirmation, EventSource, Future, Method, Reply, Service, Value};

pub(super) struct ActivityService {
    pub host: Arc<Host>,
    pub activity: watch::Sender<usize>,
}

struct Active(watch::Sender<usize>);
impl Active {
    fn new(activity: &watch::Sender<usize>) -> Self {
        activity.send_modify(|count| *count += 1);
        Self(activity.clone())
    }
}
impl Drop for Active {
    fn drop(&mut self) {
        self.0.send_modify(|count| *count -= 1);
    }
}

impl Service for ActivityService {
    fn call(&self, method: Method, value: Value) -> Future<'_, Reply> {
        Box::pin(async move {
            let active = Active::new(&self.activity);
            let mut reply = self.host.call(method, value).await?;
            let observed = reply.on_observed.take();
            reply.on_observed = Some(Box::new(ActiveConfirmation {
                observed,
                _active: active,
            }));
            reply.events = reply.events.map(|source| {
                Box::new(ActiveSource {
                    source,
                    _active: Active::new(&self.activity),
                }) as Box<dyn EventSource>
            });
            Ok(reply)
        })
    }
}

struct ActiveConfirmation {
    observed: Option<Box<dyn Confirmation>>,
    _active: Active,
}
impl Confirmation for ActiveConfirmation {
    fn confirm(mut self: Box<Self>) -> Future<'static, ()> {
        Box::pin(async move {
            if let Some(observed) = self.observed.take() {
                observed.confirm().await?;
            }
            Ok(())
        })
    }
}

struct ActiveSource {
    source: Box<dyn EventSource>,
    _active: Active,
}
impl EventSource for ActiveSource {
    fn next(&mut self) -> Future<'_, Value> {
        self.source.next()
    }
}

pub(super) async fn idle(mut activity: watch::Receiver<usize>) -> anyhow::Result<()> {
    loop {
        if *activity.borrow_and_update() > 0 {
            if activity.changed().await.is_err() {
                return Ok(());
            }
        } else {
            match tokio::time::timeout(HOST_REQUEST_TIMEOUT, activity.changed()).await {
                Ok(Ok(())) => {}
                Ok(Err(_)) => return Ok(()),
                Err(_) => anyhow::bail!(HOST_REQUEST_TIMEOUT_CONTEXT),
            }
        }
    }
}
