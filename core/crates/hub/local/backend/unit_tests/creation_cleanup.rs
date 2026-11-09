use super::fixtures::*;
use crate::creation_cleanup::{archive_reply, termination_reply};
use yolo_hub_identity::HubId;
use yolo_hub_rpc_protocol::Reply;
use yolo_hub_terminal_protocol::{
    CleanupTerminationOutcome, CreateShellResult, PublicError, SecretCapability,
};
use yolo_runtime::model::CleanupTerminationOutcome as RuntimeOutcome;

#[test]
fn cleanup_termination_maps_both_runtime_outcomes_without_inventing_success() {
    let creation = CreateShellResult {
        address: target().address,
        creation_capability: SecretCapability::generate(),
    };
    for (runtime, public) in [
        (
            RuntimeOutcome::Terminated,
            CleanupTerminationOutcome::Terminated,
        ),
        (
            RuntimeOutcome::Survivor,
            CleanupTerminationOutcome::Survivor,
        ),
    ] {
        assert_eq!(
            termination_reply(creation, runtime),
            Reply::CreationTerminated {
                address: creation.address,
                outcome: public
            }
        );
    }
}

#[test]
fn cleanup_archive_requires_origin_and_returned_session_to_match_creation() {
    let creation = CreateShellResult {
        address: target().address,
        creation_capability: SecretCapability::generate(),
    };
    let mut archived = view();
    archived.manifest.archived = true;
    let Reply::Terminal { terminal: session } =
        archive_reply(creation.address.hub_id, creation, &archived).unwrap()
    else {
        panic!()
    };
    assert_eq!(session.fields().address, creation.address);
    assert!(session.fields().archived);
    assert_eq!(
        archive_reply(HubId::new(), creation, &archived),
        Err(PublicError::OriginMismatch)
    );
    archived.manifest.archived = false;
    assert_eq!(
        archive_reply(creation.address.hub_id, creation, &archived),
        Err(PublicError::OutcomeUnknown)
    );
    archived.manifest.archived = true;
    for invalid in [uuid::Uuid::new_v4(), uuid::Uuid::nil()] {
        archived.manifest.terminal_id = invalid;
        assert_eq!(
            archive_reply(creation.address.hub_id, creation, &archived),
            Err(PublicError::OutcomeUnknown)
        );
    }
}
