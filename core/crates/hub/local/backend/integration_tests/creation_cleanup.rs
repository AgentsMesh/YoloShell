use super::{
    cleanup_rpc::*,
    fixtures::{constants::MESSAGE, view},
};
use yolo_hub_identity::HubId;
use yolo_hub_rpc_protocol::{Command, Reply};
use yolo_hub_terminal_protocol::{CleanupTerminationOutcome, PublicError, SecretCapability};
use yolo_runtime::{
    model::{
        CleanupTerminationOutcome as RuntimeOutcome, CreationCleanupArchiveRequest,
        CreationCleanupTerminateRequest,
    },
    protocol::{ErrorBody, ResponseEnvelope, error_code, method},
};

#[tokio::test]
async fn creation_termination_uses_only_exact_cleanup_capability_and_actual_reply() {
    for force in [false, true] {
        for (outcome, expected) in [
            (
                RuntimeOutcome::Terminated,
                CleanupTerminationOutcome::Terminated,
            ),
            (
                RuntimeOutcome::Survivor,
                CleanupTerminationOutcome::Survivor,
            ),
        ] {
            let fixture = ProtocolFixture::new();
            let creation = creation();
            let command = Command::CleanupTerminate { creation, force };
            let peer = fixture.reply(|request| {
                assert_eq!(request.method, method::CLEANUP_TERMINATE_SESSION);
                let payload: CreationCleanupTerminateRequest = request.decode_payload().unwrap();
                assert_eq!(payload.terminal_id, creation.address.terminal_id.as_uuid());
                assert_eq!(
                    payload.creation_capability,
                    creation.creation_capability.as_uuid()
                );
                assert_eq!(payload.force, force);
                ResponseEnvelope::success(request, &outcome).unwrap()
            });
            let (result, ()) =
                bounded(async { tokio::join!(fixture.backend.execute(&command), peer) }).await;
            assert_eq!(
                result.unwrap(),
                Reply::CreationTerminated {
                    address: creation.address,
                    outcome: expected
                }
            );
        }
    }
}

#[tokio::test]
async fn archive_cleanup_uses_exact_capability_and_rejects_substituted_session_response() {
    for (substituted, is_archived) in [(false, true), (true, true), (false, false)] {
        let fixture = ProtocolFixture::new();
        let creation = creation();
        let command = Command::CleanupArchive { creation };
        let peer = fixture.reply(|request| {
            assert_eq!(request.method, method::CLEANUP_ARCHIVE_SESSION);
            let payload: CreationCleanupArchiveRequest = request.decode_payload().unwrap();
            assert_eq!(payload.terminal_id, creation.address.terminal_id.as_uuid());
            assert_eq!(
                payload.creation_capability,
                creation.creation_capability.as_uuid()
            );
            let mut archived = view();
            archived.manifest.archived = is_archived;
            if substituted {
                archived.manifest.terminal_id = uuid::Uuid::new_v4();
            }
            ResponseEnvelope::success(request, &archived).unwrap()
        });
        let (result, ()) =
            bounded(async { tokio::join!(fixture.backend.execute(&command), peer) }).await;
        if substituted || !is_archived {
            assert_eq!(result, Err(PublicError::OutcomeUnknown));
        } else {
            let Reply::Terminal { terminal: session } = result.unwrap() else {
                panic!()
            };
            assert_eq!(session.fields().address, creation.address);
            assert!(session.fields().archived);
        }
    }
}

#[tokio::test]
async fn rejected_creation_capability_is_not_retried_using_normal_lifecycle_authority() {
    for archive in [false, true] {
        let fixture = ProtocolFixture::new();
        let mut creation = creation();
        let correct_capability = creation.creation_capability;
        creation.creation_capability = SecretCapability::generate();
        let command = if archive {
            Command::CleanupArchive { creation }
        } else {
            Command::CleanupTerminate {
                creation,
                force: true,
            }
        };
        let peer = fixture.reply(|request| {
            let supplied = if archive {
                assert_eq!(request.method, method::CLEANUP_ARCHIVE_SESSION);
                request
                    .decode_payload::<CreationCleanupArchiveRequest>()
                    .unwrap()
                    .creation_capability
            } else {
                assert_eq!(request.method, method::CLEANUP_TERMINATE_SESSION);
                request
                    .decode_payload::<CreationCleanupTerminateRequest>()
                    .unwrap()
                    .creation_capability
            };
            assert_ne!(supplied, correct_capability.as_uuid());
            assert_eq!(supplied, creation.creation_capability.as_uuid());
            ResponseEnvelope::failure(
                request,
                ErrorBody::new(error_code::UNAUTHORIZED, MESSAGE, false),
            )
        });
        let (result, ()) =
            bounded(async { tokio::join!(fixture.backend.execute(&command), peer) }).await;
        assert_eq!(result, Err(PublicError::Unauthorized));
    }
}

#[tokio::test]
async fn cross_origin_cleanup_is_rejected_without_contacting_the_runtime() {
    let fixture = ProtocolFixture::new();
    let mut creation = creation();
    creation.address.hub_id = HubId::new();
    for command in [
        Command::CleanupArchive { creation },
        Command::CleanupTerminate {
            creation,
            force: true,
        },
    ] {
        assert_eq!(
            fixture.backend.execute(&command).await,
            Err(PublicError::OriginMismatch)
        );
    }
}
