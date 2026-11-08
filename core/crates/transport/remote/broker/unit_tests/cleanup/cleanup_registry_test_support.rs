use super::super::support::FakeConnection;
use crate::remote::*;
use crate::remote_broker::{BrokerConnectFuture, BrokerConnectionFactory};
use std::future::pending;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use tokio::sync::{Notify, Semaphore};
#[path = "../../test_support/unsupported_extensions.rs"]
mod unsupported_extensions;
use unsupported_extensions::unsupported_remote_extensions;

const CLEANUP_RELEASE_PERMITS: usize = 1;
const UNBOUND_CLEANUP_FACTORY_MESSAGE: &str = "cleanup test factory requires broker binding";
const FAILED_CLEANUP_MESSAGE: &str = "injected registered cleanup failure";

#[derive(Clone, Copy)]
pub(super) enum CleanupCompletion {
    Released,
    Indeterminate,
    Failed,
}

pub(super) struct CleanupProbe {
    command_entered: AtomicBool,
    cleanup_entered: AtomicBool,
    command_notice: Notify,
    cleanup_notice: Notify,
    cleanup_release: Semaphore,
    completion: CleanupCompletion,
}

impl CleanupProbe {
    pub(super) fn new(completion: CleanupCompletion) -> Arc<Self> {
        Arc::new(Self {
            command_entered: AtomicBool::new(false),
            cleanup_entered: AtomicBool::new(false),
            command_notice: Notify::new(),
            cleanup_notice: Notify::new(),
            cleanup_release: Semaphore::new(usize::default()),
            completion,
        })
    }

    pub(super) async fn wait_command(&self) {
        wait_flag(&self.command_entered, &self.command_notice).await;
    }

    pub(super) async fn wait_cleanup(&self) {
        wait_flag(&self.cleanup_entered, &self.cleanup_notice).await;
    }

    pub(super) fn release_cleanup(&self) {
        self.cleanup_release.add_permits(CLEANUP_RELEASE_PERMITS);
    }

    async fn cleanup(self: Arc<Self>) -> Result<(), RemoteError> {
        self.cleanup_entered.store(true, Ordering::Release);
        self.cleanup_notice.notify_one();
        match self.completion {
            CleanupCompletion::Released => {
                self.cleanup_release.acquire().await.unwrap().forget();
                Ok(())
            }
            CleanupCompletion::Indeterminate => pending().await,
            CleanupCompletion::Failed => Err(RemoteError::transport(FAILED_CLEANUP_MESSAGE)),
        }
    }
}

struct CleanupCustody {
    reservation: Option<crate::remote::RemoteCleanupReservation>,
    probe: Arc<CleanupProbe>,
}

impl Drop for CleanupCustody {
    fn drop(&mut self) {
        if let Some(reservation) = self.reservation.take() {
            reservation.launch_for_broker_test(Arc::clone(&self.probe).cleanup());
        }
    }
}

pub(super) struct CleanupConnection {
    inner: Arc<FakeConnection>,
    cleanup: Mutex<Option<RemoteCleanupRegistry>>,
    probe: Arc<CleanupProbe>,
}

impl CleanupConnection {
    pub(super) fn factory(probe: Arc<CleanupProbe>) -> Arc<CleanupFactory> {
        Arc::new(CleanupFactory {
            connection: Arc::new(Self {
                inner: FakeConnection::new([]),
                cleanup: Mutex::new(None),
                probe,
            }),
        })
    }
}

pub(super) struct CleanupFactory {
    connection: Arc<CleanupConnection>,
}

impl BrokerConnectionFactory for CleanupFactory {
    fn connect_with_cleanup(&self, cleanup: RemoteCleanupRegistry) -> BrokerConnectFuture<'_> {
        *self.connection.cleanup.lock().unwrap() = Some(cleanup);
        let connection = Arc::clone(&self.connection);
        Box::pin(async move { Ok(connection as Arc<dyn MultiplexedSshConnection>) })
    }
}

impl MultiplexedSshConnection for CleanupConnection {
    unsupported_remote_extensions!();
    fn info(&self) -> &ConnectionInfo {
        self.inner.info()
    }

    fn open_runtime<'a>(
        &'a self,
        request: &'a RuntimeChannelRequest,
    ) -> RemoteFuture<'a, OpenedRemoteChannel> {
        self.inner.open_runtime(request)
    }

    fn execute<'a>(
        &'a self,
        _: &'a AuthorizedCommand,
    ) -> RemoteFuture<'a, ChannelOutcome<CommandOutput>> {
        Box::pin(async move {
            let cleanup = self
                .cleanup
                .lock()
                .unwrap()
                .clone()
                .ok_or_else(|| RemoteError::transport(UNBOUND_CLEANUP_FACTORY_MESSAGE))?;
            let _custody = CleanupCustody {
                reservation: Some(cleanup.reserve_for_broker_test()?),
                probe: Arc::clone(&self.probe),
            };
            self.probe.command_entered.store(true, Ordering::Release);
            self.probe.command_notice.notify_one();
            pending().await
        })
    }

    fn canonicalize<'a>(&'a self, path: &'a Path) -> RemoteFuture<'a, ChannelOutcome<PathBuf>> {
        self.inner.canonicalize(path)
    }

    fn read_file<'a>(
        &'a self,
        read: &'a AuthorizedFileRead,
    ) -> RemoteFuture<'a, ChannelOutcome<FileReadResult>> {
        self.inner.read_file(read)
    }

    fn stage_runtime<'a>(
        &'a self,
        request: &'a RuntimeStageRequest,
    ) -> RemoteFuture<'a, ChannelOutcome<RuntimeStageResult>> {
        self.inner.stage_runtime(request)
    }

    fn activate_runtime<'a>(
        &'a self,
        request: &'a RuntimeActivationRequest,
    ) -> RemoteFuture<'a, ChannelOutcome<()>> {
        self.inner.activate_runtime(request)
    }
}

async fn wait_flag(flag: &AtomicBool, notice: &Notify) {
    loop {
        let changed = notice.notified();
        if flag.load(Ordering::Acquire) {
            return;
        }
        changed.await;
    }
}
