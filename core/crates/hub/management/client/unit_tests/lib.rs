use super::*;
use yolo_hub_client::ClientError;
use yolo_hub_terminal_protocol::PublicError;

#[test]
fn errors_preserve_conflict_and_uncertainty_without_raw_transport_details() {
    let cases = [
        (
            ClientError::Remote(PublicError::OutcomeUnknown),
            ManagementError::OutcomeUnknown,
        ),
        (
            ClientError::OutcomeUnknownResponse(PublicError::InvalidPayload),
            ManagementError::OutcomeUnknown,
        ),
        (
            ClientError::Remote(PublicError::Conflict),
            ManagementError::Conflict,
        ),
        (ClientError::Unsupported, ManagementError::Unsupported),
        (
            ClientError::Remote(PublicError::Unsupported),
            ManagementError::Unsupported,
        ),
        (
            ClientError::Validation(PublicError::InvalidRequest),
            ManagementError::InvalidRequest,
        ),
        (
            ClientError::Remote(PublicError::Unreachable),
            ManagementError::Unavailable,
        ),
        (
            ClientError::Remote(PublicError::Unauthorized),
            ManagementError::Rejected,
        ),
        (ClientError::WrongLocalUser, ManagementError::Rejected),
        (ClientError::Poisoned, ManagementError::Unavailable),
        (ClientError::TerminalStream, ManagementError::Unavailable),
    ];
    for (input, expected) in cases {
        let actual = ManagementError::from(input);
        assert_eq!(actual, expected);
        assert!(!actual.to_string().is_empty());
    }
}

#[tokio::test]
async fn non_management_commands_are_rejected_before_socket_access() {
    let result = execute(
        Path::new("/missing.sock"),
        HubId::new(),
        Command::Hello {},
        Instant::now(),
    )
    .await;
    assert_eq!(result, Err(ManagementError::InvalidRequest));
}
