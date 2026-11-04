use crate::{fixtures::directory::*, support::NOW};
use yolo_hub_agent_protocol::{AgentCapabilities, AgentSnapshot, AgentWorkingDirectory};
use yolo_hub_peer_protocol::{ExportedOrigin, Purpose, required_rights};
use yolo_hub_routing::Right;
use yolo_hub_rpc_protocol::Command;
use yolo_hub_transport::{FRAME_LENGTH_BYTES, decode, encode};

#[test]
fn signed_agent_directory_survives_real_peer_encoding_and_two_hop_delegation() {
    let fixture = fixture(Some(AGENT_DIRECTORY));
    let encoded = encode(&fixture.export).unwrap();
    let forwarded: ExportedOrigin = decode(&encoded[FRAME_LENGTH_BYTES..]).unwrap();
    forwarded
        .verify(
            fixture.entry.identity,
            fixture.relay.identity,
            fixture.origin.identity,
            NOW,
        )
        .unwrap();
    assert_eq!(forwarded, fixture.export);
    let agent = &forwarded.catalogue.payload.agents[0];
    assert_eq!(
        agent.fields().working_directory.as_ref().unwrap().as_str(),
        AGENT_DIRECTORY
    );
    assert_eq!(
        agent.cursor().target().address.hub_id,
        fixture.origin.identity.hub()
    );
    assert_eq!(
        agent.fields().terminal.address.hub_id,
        fixture.origin.identity.hub()
    );
    assert_eq!(agent.fields().capabilities, AgentCapabilities::UNSUPPORTED);
    let rights = required_rights(&Command::GetAgent {
        target: agent.cursor().target(),
    })
    .expect("remote observation rights");
    assert_eq!(rights, [Right::Observe].into_iter().collect());
    assert!(!rights.allows(&[Right::FileRead].into_iter().collect()));
}

#[test]
fn relay_cannot_replace_or_remove_a_directory_without_breaking_the_origin_signature() {
    let fixture = fixture(Some(AGENT_DIRECTORY));
    let fingerprint = fixture.export.catalogue.fingerprint().unwrap();
    for directory in [None, Some(REPLACEMENT_DIRECTORY)] {
        let mut forged = fixture.export.clone();
        let mut fields = forged.catalogue.payload.agents[0].fields().clone();
        fields.working_directory =
            directory.map(|path| AgentWorkingDirectory::new(path.into()).unwrap());
        forged.catalogue.payload.agents[0] = AgentSnapshot::new(fields).unwrap();
        assert_ne!(forged.catalogue.fingerprint().unwrap(), fingerprint);
        assert!(
            forged
                .catalogue
                .verify(Purpose::Catalogue, fixture.origin.identity)
                .is_err()
        );
        assert!(
            forged
                .verify(
                    fixture.entry.identity,
                    fixture.relay.identity,
                    fixture.origin.identity,
                    NOW
                )
                .is_err()
        );
    }
}

#[test]
fn absent_directory_retains_valid_signature_and_cannot_be_filled_by_a_relay() {
    let fixture = fixture(None);
    let encoded = encode(&fixture.export).unwrap();
    let mut forwarded: ExportedOrigin = decode(&encoded[FRAME_LENGTH_BYTES..]).unwrap();
    forwarded
        .verify(
            fixture.entry.identity,
            fixture.relay.identity,
            fixture.origin.identity,
            NOW,
        )
        .unwrap();
    assert!(
        forwarded.catalogue.payload.agents[0]
            .fields()
            .working_directory
            .is_none()
    );
    let mut fields = forwarded.catalogue.payload.agents[0].fields().clone();
    fields.working_directory = Some(AgentWorkingDirectory::new(AGENT_DIRECTORY.into()).unwrap());
    forwarded.catalogue.payload.agents[0] = AgentSnapshot::new(fields).unwrap();
    assert!(
        forwarded
            .verify(
                fixture.entry.identity,
                fixture.relay.identity,
                fixture.origin.identity,
                NOW
            )
            .is_err()
    );
}
