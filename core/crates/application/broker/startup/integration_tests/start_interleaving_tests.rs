use super::super::{BrokerClient, BrokerConfig, BrokerPaths};
use super::fixture;
use super::support::{RuntimeFixture, running_fixture};
use crate::start::reconcile;
use std::fs;
use std::sync::Arc;
#[path = "../test_support/start_interleaving_support.rs"]
mod interleaving_support;
use interleaving_support::Spawner;

const EXPECTED_LAUNCH_COUNT: usize = 1;

fn set_port(fixture: &RuntimeFixture, port: u16) -> serde_json::Value {
    let mut value: serde_json::Value =
        serde_json::from_slice(&fs::read(&fixture.options.config_source).unwrap()).unwrap();
    value[fixture::ENDPOINT_FIELD][fixture::PORT_FIELD] = serde_json::json!(port);
    value
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn concurrent_a_and_b_reconciles_cannot_retarget_a_to_b() {
    let (fixture, old_task) = running_fixture().await;
    let desired_a = set_port(&fixture, fixture::REPLACEMENT_PORT);
    fs::write(
        &fixture.options.config_source,
        serde_json::to_vec(&desired_a).unwrap(),
    )
    .unwrap();
    let revision_a = BrokerConfig::load(&fixture.options.config_source)
        .unwrap()
        .revision()
        .to_owned();
    let desired_b = set_port(&fixture, fixture::SECOND_REPLACEMENT_PORT);
    let revision_b = {
        fs::write(
            &fixture.options.config_source,
            serde_json::to_vec(&desired_b).unwrap(),
        )
        .unwrap();
        let revision = BrokerConfig::load(&fixture.options.config_source)
            .unwrap()
            .revision()
            .to_owned();
        fs::write(
            &fixture.options.config_source,
            serde_json::to_vec(&desired_a).unwrap(),
        )
        .unwrap();
        revision
    };
    let (entered_send, entered_receive) = std::sync::mpsc::channel();
    let (release_send, release_receive) = std::sync::mpsc::channel();
    let first_spawner = Arc::new(Spawner::call_blocked(
        fixture.options.config_source.clone(),
        desired_b,
        entered_send,
        release_receive,
    ));
    let first_options = fixture.options.clone();
    let first = tokio::spawn({
        let spawner = Arc::clone(&first_spawner);
        async move { reconcile(first_options, spawner.as_ref(), fixture::START_TIMEOUT).await }
    });
    tokio::task::spawn_blocking(move || entered_receive.recv().unwrap())
        .await
        .unwrap();
    let second_spawner = Arc::new(Spawner::ready());
    let second_options = fixture.options.clone();
    let second = tokio::spawn({
        let spawner = Arc::clone(&second_spawner);
        async move { reconcile(second_options, spawner.as_ref(), fixture::START_TIMEOUT).await }
    });
    release_send.send(()).unwrap();
    first.await.unwrap().unwrap();
    second.await.unwrap().unwrap();
    assert_only_revision(&first_spawner, &revision_a);
    assert_only_revision(&second_spawner, &revision_b);
    old_task.await.unwrap().unwrap();
    let paths = BrokerPaths::new(&fixture.options.broker_state).unwrap();
    BrokerClient::new(paths.management_socket())
        .shutdown(revision_b)
        .await
        .unwrap();
    assert_eq!(
        first_spawner.successful_task_count().await,
        EXPECTED_LAUNCH_COUNT
    );
    assert_eq!(
        second_spawner.successful_task_count().await,
        EXPECTED_LAUNCH_COUNT
    );
    assert_eq!(first_spawner.calls(), EXPECTED_LAUNCH_COUNT);
    assert_eq!(second_spawner.calls(), EXPECTED_LAUNCH_COUNT);
}

fn assert_only_revision(spawner: &Spawner, expected: &str) {
    let revisions = spawner.revisions();
    assert!(!revisions.is_empty());
    assert!(revisions.iter().all(|revision| revision == expected));
}
