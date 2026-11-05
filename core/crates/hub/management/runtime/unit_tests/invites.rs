use super::*;
use yolo_hub_enrollment_protocol::{InviteDial, InvitePolicy, InviteRequest, decode};

fn invite() -> InviteRequest {
    InviteRequest {
        dial: InviteDial {
            host: "192.0.2.10".into(),
            port: std::num::NonZeroU16::new(7400).unwrap(),
        },
        lifetime_ms: 10 * 60 * 1000,
        policy: InvitePolicy {
            export_local: true,
            import_peer: false,
        },
    }
}

#[tokio::test]
async fn mint_invite_is_local_verifiable_and_does_not_touch_configuration() {
    let (_directory, hub, receiver) = fixture();
    let request = command(&hub, Command::MintInvite { request: invite() });
    let Operation::Reply(Reply::Minted { invite }) = hub.dispatch(request.clone()).await.unwrap()
    else {
        panic!()
    };
    Response::new(
        &request,
        hub.hub_id(),
        Reply::Minted {
            invite: invite.clone(),
        },
    )
    .validate_for(&request)
    .unwrap();
    let decoded = decode(&invite.code).unwrap();
    assert_eq!(decoded.claims().issuer, hub.state.peer_identity());
    assert_eq!(decoded.claims().invite_id, invite.invite_id);
    assert_eq!(decoded.claims().expires_at_ms, invite.expires_at_ms);
    assert_eq!(decoded.claims().dial.host, "192.0.2.10");
    assert!(receiver.is_empty());
}

#[tokio::test]
async fn mint_invite_is_unauthorized_for_a_remote_destination() {
    let (_directory, hub, _) = fixture();
    let request = Request::new(
        HubId::new(),
        Command::MintInvite {
            request: invite(),
        },
    )
    .unwrap();
    assert!(matches!(
        hub.dispatch(request).await,
        Err(PublicError::Unauthorized)
    ));
}
