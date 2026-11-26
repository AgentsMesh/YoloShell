use super::{cleanup_rpc::*, creation_query_support::*};
use yolo_hub_rpc_protocol::Command;
use yolo_hub_terminal_protocol::{CreationQueryRequest, PublicError};
use yolo_runtime::{
    model::{CreateTerminalResult, CreationQueryOutcome, CreationQueryResult},
    protocol::{ResponseEnvelope, method},
};

#[tokio::test]
async fn malformed_host_creation_identities_never_escape_as_query_authority() {
    let changes: &[fn(&mut CreateTerminalResult)] = &[
        |creation| creation.terminal_id = uuid::Uuid::nil(),
        |creation| creation.creation_capability = uuid::Uuid::nil(),
        |creation| creation.creation_capability = NON_RANDOM_QUERY_CAPABILITY,
    ];
    for change in changes {
        let fixture = ProtocolFixture::new();
        let original = request();
        let command = Command::QueryCreation {
            request: CreationQueryRequest::new(original.clone()),
        };
        let creation = creation();
        let mut result = CreateTerminalResult {
            terminal_id: creation.address.terminal_id.as_uuid(),
            creation_capability: creation.creation_capability.as_uuid(),
        };
        change(&mut result);
        let peer = fixture.reply(|request| {
            assert_eq!(request.method, method::QUERY_CREATION);
            ResponseEnvelope::success(
                request,
                &CreationQueryResult {
                    creation_token: original.fields().creation_token,
                    outcome: CreationQueryOutcome::Ready { creation: result },
                },
            )
            .unwrap()
        });
        let (result, ()) =
            bounded(async { tokio::join!(fixture.backend.execute(&command), peer) }).await;
        assert_eq!(result, Err(PublicError::OutcomeUnknown));
        fixture.assert_no_pending_connection();
    }
}
