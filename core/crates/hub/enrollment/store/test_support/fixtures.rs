use yolo_hub_enrollment_store::PreparedEnrollment;
use yolo_hub_federation_settings::{Configuration, ExportPolicy, Peer};
use yolo_hub_management_protocol::{
    CONFIGURATION_SCHEMA, ConfigurationChange, ConfigurationSnapshot, ConfigurationState,
};
use yolo_remote_provisioning_protocol::RemoteBootstrapReceipt;
use yolo_hub_remote_startup_test_support::*;

pub fn prepared(no_change: bool) -> PreparedEnrollment {
    let request = request();
    let peer = request.fields().expected_peer.unwrap();
    let mut configuration = Configuration::default();
    let snapshot = ConfigurationSnapshot {
        state: ConfigurationState {
            schema_version: CONFIGURATION_SCHEMA,
            identity: peer,
            revision: FIRST_REVISION,
            last_change: uuid::Uuid::new_v4(),
            configuration: configuration.clone(),
        },
        applied: true,
        peer_socket: Some("/run/user/1000/yolo/hub/peer.sock".into()),
    };
    configuration.peers.push(Peer {
        identity: request.fields().consumer,
        dial: None,
        consume: false,
        exports: vec![ExportPolicy {
            origin: peer.hub(),
            rights: request.fields().rights.clone(),
            remaining_reexports: request.fields().remaining_reexports,
        }],
    });
    let change = ConfigurationChange {
        request_id: request.fields().request_id,
        expected_revision: snapshot.state.revision,
        configuration,
    };
    let mut prepared = PreparedEnrollment {
        request,
        peer,
        snapshot,
        change: Some(change),
    };
    if no_change {
        prepared.snapshot = applied(&prepared);
        prepared.change = None;
    }
    prepared.validate().unwrap();
    prepared
}
pub fn applied(prepared: &PreparedEnrollment) -> ConfigurationSnapshot {
    let mut snapshot = prepared.snapshot.clone();
    if let Some(change) = &prepared.change {
        snapshot.state.revision = prepared.revision();
        snapshot.state.last_change = change.request_id;
        snapshot.state.configuration = change.configuration.clone();
    }
    snapshot
}
pub fn completed(prepared: &PreparedEnrollment) -> RemoteBootstrapReceipt {
    let mut fields = receipt(&prepared.request);
    fields.peer = prepared.peer;
    fields.configuration_revision = prepared.revision();
    RemoteBootstrapReceipt::new(fields).unwrap()
}
