use crate::{
    commands::*,
    fixtures::{constants::*, *},
};
use yolo_hub_identity::RuntimeEpoch;
use yolo_hub_rpc_protocol::{Command, Reply, Response};
use yolo_hub_terminal_protocol::{LeaseGrant, PublicError};

#[test]
fn session_created_and_attached_replies_cannot_change_origin_or_resource() {
    for command in [
        Command::Get {
            address: target().address,
        },
        Command::Terminal {
            command: session_command(),
        },
    ] {
        let request = request(command);
        for (origin, terminal_id) in [(SECOND_HUB_ID, SESSION_ID), (HUB_ID, SECOND_SESSION_ID)] {
            let response = Response::new(
                &request,
                hub(HUB_ID),
                Reply::Terminal {
                    terminal: summary(origin, terminal_id),
                },
            );
            assert_eq!(
                response.validate_for(&request),
                Err(PublicError::InvalidPayload)
            );
        }
    }
    let request = request(Command::Create { request: create() });
    let mut wrong_creation = creation();
    wrong_creation.address.hub_id = hub(SECOND_HUB_ID);
    assert_eq!(
        Response::new(
            &request,
            hub(HUB_ID),
            Reply::Created {
                creation: wrong_creation
            }
        )
        .validate_for(&request),
        Err(PublicError::InvalidPayload)
    );
}

#[test]
fn lease_grants_match_origin_runtime_and_both_principal_identities() {
    for command in [
        Command::Lease { request: lease() },
        Command::Renew { grant: grant() },
    ] {
        let request = request(command);
        let original = grant_fields();
        let mut wrong_origin = original;
        wrong_origin.target.address.hub_id = hub(SECOND_HUB_ID);
        let mut wrong_session = original;
        wrong_session.target.address.terminal_id = address(HUB_ID, SECOND_SESSION_ID).terminal_id;
        let mut wrong_runtime = original;
        wrong_runtime.target.runtime_epoch =
            RuntimeEpoch::from_uuid(id(SECOND_RUNTIME_ID)).unwrap();
        let mut wrong_client = original;
        wrong_client.principal.client_id = id(SECOND_REQUEST_ID);
        let mut wrong_instance = original;
        wrong_instance.principal.controller_instance_id = id(SECOND_REQUEST_ID);
        for fields in [
            wrong_origin,
            wrong_session,
            wrong_runtime,
            wrong_client,
            wrong_instance,
        ] {
            let reply = Reply::Lease {
                grant: LeaseGrant::new(fields).unwrap(),
            };
            assert_eq!(
                Response::new(&request, hub(HUB_ID), reply).validate_for(&request),
                Err(PublicError::InvalidPayload)
            );
        }
    }
}

#[test]
fn renew_cannot_replace_the_lease_generation_or_capability() {
    let request = request(Command::Renew { grant: grant() });
    let mut wrong_generation = grant_fields();
    wrong_generation.generation += STEP;
    let mut wrong_capability = grant_fields();
    wrong_capability.capability = SECOND_CAPABILITY.to_owned().try_into().unwrap();
    for fields in [wrong_generation, wrong_capability] {
        let reply = Reply::Lease {
            grant: LeaseGrant::new(fields).unwrap(),
        };
        assert_eq!(
            Response::new(&request, hub(HUB_ID), reply).validate_for(&request),
            Err(PublicError::InvalidPayload)
        );
    }
}

#[test]
fn attachment_response_is_checked_against_the_requested_runtime_and_cursor() {
    use yolo_hub_terminal_protocol::{
        AttachRequest, AttachResult, AttachResultFields, OutputCursor,
    };
    let original = attached();
    let mut other = target();
    other.address.hub_id = hub(SECOND_HUB_ID);
    let wrong_origin = AttachResult::new(AttachResultFields {
        target: other,
        snapshot: None,
        chunks: Vec::new(),
        history: None,
        next_sequence: yolo_hub_terminal_protocol::FIRST_OUTPUT_SEQUENCE,
        resynchronized: false,
    })
    .unwrap();
    let initial = request(Command::Attach { request: attach() });
    assert_eq!(
        Response::new(
            &initial,
            hub(HUB_ID),
            Reply::Attached {
                attachment: wrong_origin
            }
        )
        .validate_for(&initial),
        Err(PublicError::InvalidPayload)
    );
    let cursor = OutputCursor {
        target: target(),
        after_sequence: original.fields().next_sequence,
    };
    let later = request(Command::Attach {
        request: AttachRequest::new(target(), Some(cursor)).unwrap(),
    });
    assert_eq!(
        Response::new(
            &later,
            hub(HUB_ID),
            Reply::Attached {
                attachment: original
            }
        )
        .validate_for(&later),
        Err(PublicError::InvalidPayload)
    );
}
