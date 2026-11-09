use crate::{
    commands::*,
    fixtures::{constants::*, *},
};
use serde_json::Value;
use yolo_hub_rpc_protocol::{Command, Reply, Request, Response};
use yolo_hub_terminal_protocol::{CleanupTerminationOutcome, PublicError};

#[test]
fn cleanup_commands_are_mutations_bound_to_the_exact_origin() {
    for force in [false, true] {
        for command in [
            Command::CleanupTerminate {
                creation: creation(),
                force,
            },
            Command::CleanupArchive {
                creation: creation(),
            },
        ] {
            assert!(command.mutates());
            assert_eq!(command.request_id(), None);
            assert!(Request::new(hub(HUB_ID), command.clone()).is_ok());
            assert_eq!(
                Request::new(hub(SECOND_HUB_ID), command),
                Err(PublicError::OriginMismatch)
            );
        }
    }
}

#[test]
fn termination_reply_preserves_survivor_and_never_echoes_creation_capability() {
    let request = request(Command::CleanupTerminate {
        creation: creation(),
        force: true,
    });
    for outcome in [
        CleanupTerminationOutcome::Terminated,
        CleanupTerminationOutcome::Survivor,
    ] {
        let reply = Reply::CreationTerminated {
            address: creation().address,
            outcome,
        };
        assert!(
            Response::new(&request, hub(HUB_ID), reply.clone())
                .validate_for(&request)
                .is_ok()
        );
        let wire = serde_json::to_value(&reply).unwrap();
        assert!(wire.get(CREATION_CAPABILITY_FIELD).is_none());
        assert!(wire.get(CREATION_FIELD).is_none());
        assert!(!serde_json::to_string(&reply).unwrap().contains(CAPABILITY));
        for field in [CREATION_CAPABILITY_FIELD, CREATION_FIELD] {
            let mut injected = wire.clone();
            injected[field] = Value::String(CAPABILITY.into());
            assert!(serde_json::from_value::<Reply>(injected).is_err());
        }
        for wrong in [
            address(SECOND_HUB_ID, SESSION_ID),
            address(HUB_ID, SECOND_SESSION_ID),
        ] {
            let reply = Reply::CreationTerminated {
                address: wrong,
                outcome,
            };
            assert_eq!(
                Response::new(&request, hub(HUB_ID), reply).validate_for(&request),
                Err(PublicError::InvalidPayload)
            );
        }
        let mut unknown = wire;
        unknown[CLEANUP_OUTCOME_FIELD] = Value::String(UNKNOWN_TEXT.into());
        assert!(serde_json::from_value::<Reply>(unknown).is_err());
    }
}

#[test]
fn archive_cleanup_response_cannot_substitute_another_session() {
    let request = request(Command::CleanupArchive {
        creation: creation(),
    });
    for (origin, session) in [
        (HUB_ID, SESSION_ID),
        (HUB_ID, SECOND_SESSION_ID),
        (SECOND_HUB_ID, SESSION_ID),
    ] {
        let response = Response::new(
            &request,
            hub(HUB_ID),
            Reply::Terminal {
                terminal: archived_summary(origin, session),
            },
        );
        assert_eq!(
            response.validate_for(&request).is_ok(),
            origin == HUB_ID && session == SESSION_ID
        );
    }
    assert_eq!(
        Response::new(&request, hub(HUB_ID), Reply::Completed {}).validate_for(&request),
        Err(PublicError::InvalidPayload)
    );
    assert_eq!(
        Response::new(
            &request,
            hub(HUB_ID),
            Reply::Terminal {
                terminal: summary(HUB_ID, SESSION_ID),
            }
        )
        .validate_for(&request),
        Err(PublicError::InvalidPayload)
    );
}

#[test]
fn cleanup_capability_and_creation_payload_fields_are_strict() {
    for command in [
        Command::CleanupTerminate {
            creation: creation(),
            force: false,
        },
        Command::CleanupArchive {
            creation: creation(),
        },
    ] {
        let wire = serde_json::to_value(command).unwrap();
        for invalid in [
            CAPABILITY.to_uppercase(),
            capability().as_uuid().simple().to_string(),
            uuid::Uuid::nil().to_string(),
            id(REQUEST_ID).to_string(),
            UNKNOWN_TEXT.into(),
        ] {
            let mut replaced = wire.clone();
            replaced[CREATION_FIELD][CREATION_CAPABILITY_FIELD] = Value::String(invalid);
            assert!(serde_json::from_value::<Command>(replaced).is_err());
        }
        for field in [CREATION_CAPABILITY_FIELD, REPLY_ADDRESS_FIELD] {
            let mut missing = wire.clone();
            missing[CREATION_FIELD]
                .as_object_mut()
                .unwrap()
                .remove(field);
            assert!(serde_json::from_value::<Command>(missing).is_err());
        }
        let mut unknown = wire;
        unknown[CREATION_FIELD][UNKNOWN_FIELD] = Value::Bool(true);
        assert!(serde_json::from_value::<Command>(unknown).is_err());
    }
    let mut missing_force = serde_json::to_value(Command::CleanupTerminate {
        creation: creation(),
        force: false,
    })
    .unwrap();
    missing_force
        .as_object_mut()
        .unwrap()
        .remove(CLEANUP_FORCE_FIELD);
    assert!(serde_json::from_value::<Command>(missing_force).is_err());
}
