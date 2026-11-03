use super::{cleanup_rpc::*, creation_query_support::*};
use yolo_hub_rpc_protocol::{Command, Reply};
use yolo_hub_terminal_protocol::{CreationQueryOutcome, CreationQueryRequest, PublicError};
use yolo_runtime::{
    model::{
        CreateTerminal, CreateTerminalResult, CreationQueryOutcome as HostOutcome,
        CreationQueryResult as HostQuery,
    },
    protocol::{ErrorBody, ResponseEnvelope, error_code, method},
};

#[tokio::test]
async fn create_and_query_preserve_the_same_pre_resolution_host_request() {
    let original = request();
    let expected = creation();
    let created = CreateTerminalResult {
        terminal_id: expected.address.terminal_id.as_uuid(),
        creation_capability: expected.creation_capability.as_uuid(),
    };
    for query in [false, true] {
        let fixture = ProtocolFixture::new();
        let command = if query {
            Command::QueryCreation {
                request: CreationQueryRequest::new(original.clone()),
            }
        } else {
            Command::Create {
                request: original.clone(),
            }
        };
        let peer = fixture.reply(|request| {
            assert_eq!(
                request.method,
                if query {
                    method::QUERY_CREATION
                } else {
                    method::CREATE_SESSION
                }
            );
            assert_host_request(
                &request.decode_payload::<CreateTerminal>().unwrap(),
                &original,
            );
            if query {
                ResponseEnvelope::success(
                    request,
                    &HostQuery {
                        creation_token: original.fields().creation_token,
                        outcome: HostOutcome::Ready { creation: created },
                    },
                )
                .unwrap()
            } else {
                ResponseEnvelope::success(request, &created).unwrap()
            }
        });
        let (result, ()) =
            bounded(async { tokio::join!(fixture.backend.execute(&command), peer) }).await;
        if query {
            let Reply::CreationQuery { query } = result.unwrap() else {
                panic!()
            };
            assert_eq!(
                query.outcome(),
                &CreationQueryOutcome::Ready { creation: expected }
            );
        } else {
            assert_eq!(result.unwrap(), Reply::Created { creation: expected });
        }
        fixture.assert_no_pending_connection();
    }
}

#[tokio::test]
async fn query_progresses_from_missing_and_preserves_each_origin_outcome_without_replay() {
    let fixture = ProtocolFixture::new();
    let original = request();
    let creation = creation();
    let created = CreateTerminalResult {
        terminal_id: creation.address.terminal_id.as_uuid(),
        creation_capability: creation.creation_capability.as_uuid(),
    };
    let command = Command::QueryCreation {
        request: CreationQueryRequest::new(original.clone()),
    };
    for (outcome, expected) in [
        (HostOutcome::Missing {}, CreationQueryOutcome::Missing {}),
        (HostOutcome::Pending {}, CreationQueryOutcome::Pending {}),
        (
            HostOutcome::Ready { creation: created },
            CreationQueryOutcome::Ready { creation },
        ),
        (
            HostOutcome::Finalized { creation: created },
            CreationQueryOutcome::Finalized { creation },
        ),
        (
            HostOutcome::Survivor { creation: created },
            CreationQueryOutcome::Survivor { creation },
        ),
        (
            HostOutcome::Archived {
                terminal_id: created.terminal_id,
            },
            CreationQueryOutcome::Archived {
                address: creation.address,
            },
        ),
    ] {
        let peer = fixture.reply(|request| {
            assert_eq!(request.method, method::QUERY_CREATION);
            ResponseEnvelope::success(
                request,
                &HostQuery {
                    creation_token: original.fields().creation_token,
                    outcome,
                },
            )
            .unwrap()
        });
        let (result, ()) =
            bounded(async { tokio::join!(fixture.backend.execute(&command), peer) }).await;
        let Reply::CreationQuery { query } = result.unwrap() else {
            panic!()
        };
        assert_eq!(query.outcome(), &expected);
        assert_eq!(query.creation_token(), original.fields().creation_token);
        fixture.assert_no_pending_connection();
    }
}

#[tokio::test]
async fn a_query_reply_for_another_token_never_exposes_creation_authority() {
    let fixture = ProtocolFixture::new();
    let command = Command::QueryCreation {
        request: CreationQueryRequest::new(request()),
    };
    let creation = creation();
    let peer = fixture.reply(|request| {
        assert_eq!(request.method, method::QUERY_CREATION);
        ResponseEnvelope::success(
            request,
            &HostQuery {
                creation_token: uuid::Uuid::new_v4(),
                outcome: HostOutcome::Ready {
                    creation: CreateTerminalResult {
                        terminal_id: creation.address.terminal_id.as_uuid(),
                        creation_capability: creation.creation_capability.as_uuid(),
                    },
                },
            },
        )
        .unwrap()
    });
    let (result, ()) =
        bounded(async { tokio::join!(fixture.backend.execute(&command), peer) }).await;
    assert_eq!(result, Err(PublicError::OutcomeUnknown));
    fixture.assert_no_pending_connection();
}

#[tokio::test]
async fn legacy_host_query_rejection_does_not_fall_back_to_create() {
    let fixture = ProtocolFixture::new();
    let command = Command::QueryCreation {
        request: CreationQueryRequest::new(request()),
    };
    let peer = fixture.reply(|request| {
        assert_eq!(request.method, method::QUERY_CREATION);
        ResponseEnvelope::failure(
            request,
            ErrorBody::new(
                error_code::UNSUPPORTED_METHOD,
                UNSUPPORTED_QUERY_MESSAGE,
                false,
            ),
        )
    });
    let (result, ()) =
        bounded(async { tokio::join!(fixture.backend.execute(&command), peer) }).await;
    assert_eq!(result, Err(PublicError::Unsupported));
    fixture.assert_no_pending_connection();
}
