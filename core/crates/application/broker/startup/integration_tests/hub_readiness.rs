use super::{fixture, hub_start_fixture::*, ready_remote};
use crate::pending::PendingSpawnHandle;
use crate::{
    Result,
    start::{BrokerSpawner, reconcile},
};
use std::sync::atomic::{AtomicUsize, Ordering};
use yolo_broker_supervision_state::ChildLaunch;
use yolo_remote_broker::{
    BrokerConfig, BrokerPaths, BrokerReconcileGuard, BrokerRetirement, RemoteBroker,
    stage_broker_config,
};

const UNEXPECTED_SPAWN: &str = "adopted Hub broker was respawned";
const SPAWN_INCREMENT: usize = 1;

#[derive(Default)]
struct NoSpawn(AtomicUsize);
impl BrokerSpawner for NoSpawn {
    fn spawn(&self, _: &ChildLaunch, _: Option<&str>) -> Result<PendingSpawnHandle> {
        self.0.fetch_add(SPAWN_INCREMENT, Ordering::SeqCst);
        anyhow::bail!(UNEXPECTED_SPAWN)
    }
}

#[tokio::test]
async fn pending_hub_prepares_before_adoption_and_is_reused_without_host_calls() {
    let fixture = fixture();
    let config = BrokerConfig::load(&fixture.options.config_source).unwrap();
    assert!(config.hub_transport());
    let paths = BrokerPaths::new(&fixture.options.broker_state).unwrap();
    let generation = stage_broker_config(&fixture.options.config_source, &paths).unwrap();
    assert!(generation.hub_transport());
    let (factory, calls) = ready_remote::counted_factory();
    let broker = RemoteBroker::with_factory(config.clone(), paths.clone(), factory);
    let (adopt, adoption) = tokio::sync::oneshot::channel();
    let (stop, stopped) = tokio::sync::oneshot::channel::<()>();
    let task = tokio::spawn(broker.serve_supervised_until(adoption, async {
        let _ = stopped.await;
    }));
    let abort = AbortOnDrop(task.abort_handle());
    let client = client(&fixture);
    let expected = wait_ready(&client).await;
    assert!(!expected.adopted);
    let deadline = tokio::time::Instant::now() + fixture::START_TIMEOUT;
    let guard = BrokerReconcileGuard::acquire(&paths, deadline)
        .await
        .unwrap();
    let observed = BrokerRetirement::capture(&guard).unwrap();
    let binding = observed
        .prepare_hub_transport(&client, &guard, profile(&config), deadline)
        .await
        .unwrap();
    assert_eq!(binding.broker_instance_id, expected.instance_id);
    calls.assert_ssh_only(EXPECTED_CONNECTIONS);
    assert!(!client.status().await.unwrap().adopted);
    drop(guard);
    adopt
        .send(Box::new(move |instance| instance == expected.instance_id))
        .ok()
        .unwrap();
    tokio::time::timeout_at(deadline, async {
        while !client.status().await.unwrap().adopted {
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
    let spawner = NoSpawn::default();
    let ready = reconcile(fixture.options.clone(), &spawner, fixture::START_TIMEOUT)
        .await
        .unwrap();
    let reused = reconcile(fixture.options.clone(), &spawner, fixture::START_TIMEOUT)
        .await
        .unwrap();
    assert_eq!(ready, reused);
    assert!(ready.adopted);
    assert_eq!(ready.instance_id, binding.broker_instance_id);
    assert_eq!(ready.connection_id, Some(binding.connection_id));
    assert_eq!(ready.connection_generation, binding.connection_generation);
    assert_eq!(spawner.0.load(Ordering::SeqCst), usize::default());
    calls.assert_ssh_only(EXPECTED_CONNECTIONS);
    client.shutdown(config.revision().into()).await.unwrap();
    tokio::time::timeout(fixture::START_TIMEOUT, task)
        .await
        .unwrap()
        .unwrap()
        .unwrap();
    drop((abort, stop));
}

struct AbortOnDrop(tokio::task::AbortHandle);
impl Drop for AbortOnDrop {
    fn drop(&mut self) {
        self.0.abort();
    }
}
