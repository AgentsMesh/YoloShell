use anyhow::Result;
use std::future::Future;
use std::pin::Pin;

pub(crate) type MaintenanceShutdown<'a> = Pin<Box<dyn Future<Output = Result<()>> + Send + 'a>>;

pub(crate) trait MaintenanceLifecycle: Send {
    fn request_shutdown(&self);
    fn shutdown(&mut self) -> MaintenanceShutdown<'_>;
}

pub(super) struct BrokerMaintenance {
    lifecycle: Box<dyn MaintenanceLifecycle>,
}

impl BrokerMaintenance {
    pub(super) fn runtime() -> Result<Self> {
        let plan = crate::maintenance::maintenance_plan()?;
        Ok(Self::new(crate::maintenance::MaintenanceWorker::start(
            plan,
        )))
    }

    #[cfg(test)]
    pub(super) fn disabled() -> Self {
        Self::new(DisabledMaintenance)
    }

    #[cfg(test)]
    pub(crate) fn test(lifecycle: impl MaintenanceLifecycle + 'static) -> Self {
        Self::new(lifecycle)
    }

    fn new(lifecycle: impl MaintenanceLifecycle + 'static) -> Self {
        Self {
            lifecycle: Box::new(lifecycle),
        }
    }

    pub(super) fn request_shutdown(&self) {
        self.lifecycle.request_shutdown();
    }

    pub(super) async fn shutdown(&mut self) -> Result<()> {
        self.lifecycle.shutdown().await
    }
}

impl MaintenanceLifecycle for crate::maintenance::MaintenanceWorker {
    fn request_shutdown(&self) {
        crate::maintenance::MaintenanceWorker::request_shutdown(self);
    }

    fn shutdown(&mut self) -> MaintenanceShutdown<'_> {
        Box::pin(crate::maintenance::MaintenanceWorker::shutdown(self))
    }
}

#[cfg(test)]
struct DisabledMaintenance;

#[cfg(test)]
impl MaintenanceLifecycle for DisabledMaintenance {
    fn request_shutdown(&self) {}

    fn shutdown(&mut self) -> MaintenanceShutdown<'_> {
        Box::pin(std::future::ready(Ok(())))
    }
}
