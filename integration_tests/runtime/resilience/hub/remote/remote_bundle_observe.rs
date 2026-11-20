use super::{constants::*, fixture::Fixture};
use yolo_hub_build_protocol::ServiceBuild;
use yolo_hub_client::Client;
use yolo_hub_management_protocol::ConfigurationSnapshot;
use yolo_remote_provisioning_protocol::RemoteBootstrapReceipt;
use yolo_hub_rpc_protocol::{Command, Method, Reply};

pub async fn configuration(
    fixture: &Fixture,
    receipt: &RemoteBootstrapReceipt,
) -> ConfigurationSnapshot {
    let connection = fixture.connection(receipt.fields().peer);
    let mut client =
        Client::connect_local(&connection.socket_path, connection.entry_hub, deadline())
            .await
            .unwrap();
    assert!(client.hello().methods.contains(&Method::GetBuild));
    let Reply::Configuration { snapshot } = client
        .call(
            connection.entry_hub,
            Command::GetConfiguration {},
            deadline(),
        )
        .await
        .unwrap()
    else {
        panic!()
    };
    snapshot.validate().unwrap();
    assert!(snapshot.applied);
    assert_eq!(snapshot.state.identity, receipt.fields().peer);
    assert_eq!(snapshot.listen_address, Some(receipt.fields().endpoint));
    assert_eq!(
        snapshot.state.revision,
        receipt.fields().configuration_revision
    );
    let policy = snapshot
        .state
        .configuration
        .peers
        .iter()
        .find(|peer| peer.identity == fixture.consumer)
        .unwrap();
    assert!(!policy.consume && policy.dial.is_none());
    let export = policy
        .exports
        .iter()
        .find(|export| export.origin == connection.entry_hub)
        .unwrap();
    assert_eq!(export.rights, receipt.fields().rights);
    assert_eq!(
        export.remaining_reexports,
        receipt.fields().remaining_reexports
    );
    snapshot
}

pub async fn build(fixture: &Fixture, receipt: &RemoteBootstrapReceipt) -> ServiceBuild {
    let connection = fixture.connection(receipt.fields().peer);
    let mut client =
        Client::connect_local(&connection.socket_path, connection.entry_hub, deadline())
            .await
            .unwrap();
    let Reply::Build { build } = client
        .call(connection.entry_hub, Command::GetBuild {}, deadline())
        .await
        .unwrap()
    else {
        panic!()
    };
    assert_eq!(build, receipt.fields().build);
    let direct = yolo_client::Client::managed_local(
        &fixture.home,
        crate::Paths::new(&fixture.home).host_socket().unwrap(),
    )
    .unwrap();
    assert_eq!(direct.build_image().await.unwrap(), build.host);
    build
}

pub async fn same_processes(fixture: &Fixture, receipt: &RemoteBootstrapReceipt) {
    let actual = crate::hub::local_startup_support::Services::inspect(
        &fixture.home,
        &fixture.connection(receipt.fields().peer),
    )
    .await;
    assert_eq!(actual, fixture.identities());
    assert!(actual.0.matches() && actual.1.matches());
}
