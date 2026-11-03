use super::{
    constants::*, control, files, graph::Graph, observe, process::Publisher, reconnect, requests,
    shell, tui,
};
use crate::hub::federation_support::TEMP_ROOT;
use yolo_hub_agent_protocol::{AgentWorkingDirectory, Freshness};
use yolo_hub_identity::ConnectorInstanceId;

#[tokio::test(flavor = "multi_thread")]
async fn real_connector_relay_reconnect_and_revocation_keep_shell_custody() {
    let a_host = crate::agent_recovery::federation_harness().await;
    let b_host = crate::agent_recovery::federation_harness().await;
    let c_host = crate::agent_recovery::federation_harness().await;
    let root = tempfile::tempdir_in(TEMP_ROOT).unwrap();
    let graph = Graph::start(
        &root.path().canonicalize().unwrap(),
        &a_host,
        &b_host,
        &c_host,
    )
    .await;
    shell::create(&graph.a, &a_host).await;
    shell::create(&graph.b, &b_host).await;
    let target = shell::create(graph.c.running(), &c_host).await;
    let before = c_host
        .client()
        .get_session(target.address.terminal_id.as_uuid())
        .await
        .unwrap();
    let private = c_host.home.path().join(PRIVATE_MARKER);
    std::fs::write(&private, PRIVATE_CONTENT).unwrap();
    let enrollment = requests::enroll(&graph.c, target).await;
    let instance = ConnectorInstanceId::new();
    let mut publisher = Publisher::start(&graph.c.socket, target.address.hub_id).await;
    let active = requests::activate_publisher(&mut publisher, &enrollment, instance).await;
    let mut publication = requests::publication(active, active.next_sequence);
    publication.state.working_directory = Some(
        AgentWorkingDirectory::new(
            c_host
                .home
                .path()
                .canonicalize()
                .unwrap()
                .to_str()
                .unwrap()
                .into(),
        )
        .unwrap(),
    );
    let ack = requests::publish(&mut publisher, publication).await;
    assert!(!ack.duplicate);
    let origin = observe::agent(graph.c.running(), enrollment.agent, Freshness::Current).await;
    for node in [&graph.a, &graph.b, graph.c.running()] {
        node.wait_origins(&[target.address.hub_id]).await;
        let observed = observe::agent(node, enrollment.agent, Freshness::Current).await;
        observe::same(&origin, &observed);
        observe::bound(node, &observed).await;
    }
    tui::exercise(&graph, enrollment.agent).await;
    files::denied(&graph).await;
    control::unsupported(graph.c.running(), &origin, &private).await;
    shell::survives(&c_host, target, &before).await;
    publisher.crash().await;
    for node in [graph.c.running(), &graph.b, &graph.a] {
        let stale = observe::agent(node, enrollment.agent, Freshness::Stale).await;
        observe::same(&origin, &stale);
    }
    shell::ping(&graph.a, target).await;
    shell::survives(&c_host, target, &before).await;
    let mut publisher = Publisher::start(&graph.c.socket, target.address.hub_id).await;
    let resumed = requests::activate_publisher(&mut publisher, &enrollment, instance).await;
    assert_eq!(resumed.agent, active.agent);
    assert_ne!(resumed.generation, active.generation);
    let new_ack = requests::publish(
        &mut publisher,
        requests::publication(resumed, resumed.next_sequence),
    )
    .await;
    assert_ne!(new_ack.cursor.stream(), ack.cursor.stream());
    let (mut publisher, latest) =
        reconnect::replace(&graph.c, &enrollment, instance, publisher, resumed).await;
    let latest_observation =
        observe::agent(graph.c.running(), enrollment.agent, Freshness::Current).await;
    for node in [&graph.a, &graph.b, graph.c.running()] {
        let observed = observe::at(node, latest_observation.cursor()).await;
        assert_eq!(observed.cursor().target(), latest.agent);
        observe::bound(node, &observed).await;
    }
    reconnect::revoke(&graph.c, &enrollment, &mut publisher, latest).await;
    for node in [graph.c.running(), &graph.b, &graph.a] {
        observe::agent(node, enrollment.agent, Freshness::Stale).await;
    }
    files::denied(&graph).await;
    assert_eq!(std::fs::read(private).unwrap(), PRIVATE_CONTENT);
    shell::ping(&graph.a, target).await;
    shell::survives(&c_host, target, &before).await;
    publisher.stop().await;
    graph.stop().await;
}
