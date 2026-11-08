use super::super::*;
use super::support::*;
use crate::remote_broker::maintenance_runtime::{
    BrokerMaintenance, MaintenanceLifecycle, MaintenanceShutdown,
};
use std::future::pending;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;
use tokio::sync::Notify;

const EVENT_TIMEOUT: Duration = Duration::from_secs(2);
const MAINTENANCE_TEST_CONTEXT: &str = "maintenance-test-context";
const MAINTENANCE_TEST_ERROR: &str = "maintenance-test-error";
const NAMESPACE_TEST_CONTEXT: &str = "namespace-test-context";
const NAMESPACE_TEST_ERROR: &str = "namespace-test-error";
const STATE_DIRECTORY: &str = crate::remote_broker::constants::TEST_STATE_DIRECTORY;

#[derive(Debug)]
struct ShutdownTestError(&'static str);

impl std::fmt::Display for ShutdownTestError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.0)
    }
}

impl std::error::Error for ShutdownTestError {}

#[derive(Default)]
struct MaintenanceProbe {
    requested: AtomicBool,
    shutdown_entered: Notify,
    release: Notify,
}

struct BlockingMaintenance(Arc<MaintenanceProbe>);

impl MaintenanceLifecycle for BlockingMaintenance {
    fn request_shutdown(&self) {
        self.0.requested.store(true, Ordering::Release);
    }

    fn shutdown(&mut self) -> MaintenanceShutdown<'_> {
        let probe = Arc::clone(&self.0);
        Box::pin(async move {
            assert!(probe.requested.load(Ordering::Acquire));
            probe.shutdown_entered.notify_one();
            probe.release.notified().await;
            Err(anyhow::anyhow!(MAINTENANCE_TEST_ERROR))
        })
    }
}

#[test]
fn maintenance_shutdown_errors_keep_context_and_namespace_failures() {
    let maintenance = super::super::server::shutdown_result(
        super::super::server::BrokerShutdownCause::External,
        Err(anyhow::anyhow!(MAINTENANCE_TEST_ERROR)),
    )
    .unwrap_err();
    assert!(
        maintenance
            .to_string()
            .contains(super::super::server::BROKER_MAINTENANCE_ERROR_CONTEXT)
    );
    assert!(format!("{maintenance:#}").contains(MAINTENANCE_TEST_ERROR));

    let namespace =
        anyhow::Error::new(ShutdownTestError(NAMESPACE_TEST_ERROR)).context(NAMESPACE_TEST_CONTEXT);
    let maintenance = anyhow::Error::new(ShutdownTestError(MAINTENANCE_TEST_ERROR))
        .context(MAINTENANCE_TEST_CONTEXT);
    let combined = super::super::server::shutdown_result(
        super::super::server::BrokerShutdownCause::NamespaceLost(namespace),
        Err(maintenance),
    )
    .unwrap_err();
    let aggregate = combined
        .downcast_ref::<super::super::BrokerShutdownErrors>()
        .unwrap();
    assert_eq!(
        aggregate
            .namespace_error()
            .downcast_ref::<ShutdownTestError>()
            .map(|error| error.0),
        Some(NAMESPACE_TEST_ERROR)
    );
    assert_eq!(
        aggregate
            .maintenance_error()
            .downcast_ref::<ShutdownTestError>()
            .map(|error| error.0),
        Some(MAINTENANCE_TEST_ERROR)
    );
    let source = std::error::Error::source(aggregate).unwrap();
    assert_eq!(source.to_string(), NAMESPACE_TEST_CONTEXT);
    assert_eq!(
        source.source().map(ToString::to_string),
        Some(NAMESPACE_TEST_ERROR.to_owned())
    );
    let display = combined.to_string();
    for expected in [
        NAMESPACE_TEST_CONTEXT,
        NAMESPACE_TEST_ERROR,
        MAINTENANCE_TEST_CONTEXT,
        MAINTENANCE_TEST_ERROR,
    ] {
        assert!(display.contains(expected));
    }
}

#[tokio::test]
async fn rotation_releases_instance_before_maintenance_settles() {
    let root = tempfile::tempdir().unwrap();
    let paths = BrokerPaths::new(root.path().join(STATE_DIRECTORY)).unwrap();
    let broker =
        RemoteBroker::with_factory(test_config(), paths.clone(), FakeFactory::empty(Vec::new()));
    let probe = Arc::new(MaintenanceProbe::default());
    let maintenance = BrokerMaintenance::test(BlockingMaintenance(Arc::clone(&probe)));
    let task = tokio::spawn(broker.serve_until_with_maintenance(pending(), maintenance));
    wait_ready(&paths).await;

    BrokerClient::new(paths.management_socket())
        .shutdown(CONFIG_REVISION.into())
        .await
        .unwrap();
    tokio::time::timeout(EVENT_TIMEOUT, probe.shutdown_entered.notified())
        .await
        .unwrap();
    wait_available(&paths).await;
    let replacement = super::super::sockets::BrokerSockets::bind(&paths)
        .await
        .unwrap();
    probe.release.notify_one();
    let error = tokio::time::timeout(EVENT_TIMEOUT, task)
        .await
        .unwrap()
        .unwrap()
        .unwrap_err();
    assert!(
        error
            .to_string()
            .contains(super::super::server::BROKER_MAINTENANCE_ERROR_CONTEXT)
    );
    assert!(paths.management_socket().exists());
    drop(replacement);
}

async fn wait_ready(paths: &BrokerPaths) {
    let client = BrokerClient::new(paths.management_socket());
    tokio::time::timeout(EVENT_TIMEOUT, async {
        loop {
            if client.status().await.is_ok() {
                return;
            }
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
}

async fn wait_available(paths: &BrokerPaths) {
    tokio::time::timeout(EVENT_TIMEOUT, async {
        while !broker_instance_available(paths).unwrap() {
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
}
