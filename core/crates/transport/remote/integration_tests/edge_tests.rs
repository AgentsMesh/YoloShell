use super::super::*;
use crate::protocol::{ErrorBody, ProtocolError, RequestEnvelope, ResponseEnvelope, to_value};

#[test]
fn endpoint_and_host_key_accessors_preserve_validated_values() {
    for (user, host, port) in [
        ("user@name", "host", 22),
        ("user", "host/name", 22),
        ("user", "[host]", 22),
        ("user", "", 22),
        ("user", "h\u{f6}st", 22),
    ] {
        assert!(RemoteEndpoint::new(user, host, port).is_err());
    }
    let endpoint = RemoteEndpoint::new("user", "HOST.EXAMPLE", 2200).unwrap();
    assert_eq!(endpoint.user(), "user");
    assert_eq!(endpoint.host(), "host.example");
    assert_eq!(endpoint.port(), 2200);
    assert_eq!(endpoint.authority(), "user@host.example:2200");
    assert!(HostKey::new("bad algorithm", "encoded").is_err());
    assert!(HostKey::new("ssh-ed25519", "bad key").is_err());
}

#[test]
fn response_errors_and_connection_loss_are_classified_without_defaults() {
    let request =
        RequestEnvelope::new("id", crate::protocol::method::HELLO, to_value(&()).unwrap());
    let failure = ResponseEnvelope::failure(&request, ErrorBody::invalid("denied"));
    assert!(matches!(
        super::super::rpc::decode_response::<()>(failure),
        Err(RemoteError::Host(_))
    ));
    let mut missing = ResponseEnvelope::failure(&request, ErrorBody::invalid("ignored"));
    missing.error = None;
    assert!(matches!(
        super::super::rpc::decode_response::<()>(missing),
        Err(RemoteError::Host(_))
    ));
    let losses = [
        RemoteError::transport("lost"),
        RemoteError::Protocol(ProtocolError::EndOfStream),
        RemoteError::Protocol(ProtocolError::Io(std::io::Error::other("lost"))),
    ];
    assert!(losses.iter().all(RemoteError::is_connection_loss));
    assert!(!RemoteError::Disconnected.is_connection_loss());
}

#[test]
fn channel_identity_checks_connection_and_purpose_independently() {
    let connection_id = uuid::Uuid::new_v4();
    let wrong_connection = ChannelIdentity {
        connection_id: uuid::Uuid::new_v4(),
        channel_id: 1,
        purpose: ChannelPurpose::Command,
    };
    assert!(
        super::super::transport::validate_channel(
            wrong_connection,
            connection_id,
            ChannelPurpose::Command
        )
        .is_err()
    );
    let wrong_purpose = ChannelIdentity {
        connection_id,
        channel_id: 2,
        purpose: ChannelPurpose::FileRead,
    };
    assert!(
        super::super::transport::validate_channel(
            wrong_purpose,
            connection_id,
            ChannelPurpose::Command
        )
        .is_err()
    );
}
