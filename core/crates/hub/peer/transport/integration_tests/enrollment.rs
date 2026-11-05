use crate::support::*;
use ring::{
    rand::SystemRandom,
    signature::{Ed25519KeyPair, KeyPair},
};
use std::{num::NonZeroU16, sync::Arc};
use tokio::{net::TcpListener, time::timeout_at};
use yolo_hub_delegation::{DelegationError, ED25519_SIGNATURE_BYTES, PeerIdentity, Signer};
use yolo_hub_enrollment_protocol::{
    InviteClaims, InviteDial, InvitePolicy, SignedInvite, encode,
};
use yolo_hub_enrollment_wire::{
    JOIN_SCHEMA_VERSION, JoinOutcome, JoinReply, JoinRequest, read_request, write_outcome,
};

pub(crate) struct InviteSigner {
    identity: PeerIdentity,
    key: Ed25519KeyPair,
}

impl Signer for InviteSigner {
    fn identity(&self) -> PeerIdentity {
        self.identity
    }
    fn sign(&self, message: &[u8]) -> Result<[u8; ED25519_SIGNATURE_BYTES], DelegationError> {
        self.key
            .sign(message)
            .as_ref()
            .try_into()
            .map_err(|_| DelegationError::Signing)
    }
}

pub(crate) struct Verifier(SignedInvite);

impl Verifier {
    pub(crate) fn new(invite: SignedInvite) -> Self {
        Self(invite)
    }
}

impl yolo_hub_peer_transport::EnrollmentVerifier for Verifier {
    fn verify(&self, code: &str) -> Option<SignedInvite> {
        (encode(&self.0).ok().as_deref() == Some(code)).then(|| self.0.clone())
    }
}

pub(crate) fn hub() -> (InviteSigner, Arc<russh::keys::PrivateKey>) {
    let document = Ed25519KeyPair::generate_pkcs8(&SystemRandom::new()).unwrap();
    let key = Ed25519KeyPair::from_pkcs8(document.as_ref()).unwrap();
    let identity = PeerIdentity::new(
        yolo_hub_identity::HubId::new(),
        key.public_key().as_ref().try_into().unwrap(),
    )
    .unwrap();
    let transport = russh::keys::pkcs8::decode_pkcs8(document.as_ref(), None).unwrap();
    (InviteSigner { identity, key }, Arc::new(transport))
}

pub(crate) fn invite(signer: &InviteSigner, port: NonZeroU16) -> SignedInvite {
    SignedInvite::issue(
        InviteClaims {
            invite_id: uuid::Uuid::new_v4(),
            issuer: signer.identity,
            dial: Some(InviteDial {
                host: "127.0.0.1".into(),
                port,
            }),
            not_before_ms: 0,
            expires_at_ms: 60 * 60 * 1000,
            policy: InvitePolicy {
                export_local: true,
                import_peer: false,
            },
        },
        signer,
    )
    .unwrap()
}

#[tokio::test]
async fn enrollment_authenticates_with_the_code_and_returns_a_join_reply() {
    let (signer, transport) = hub();
    let listener = TcpListener::bind(("127.0.0.1", 0)).await.unwrap();
    let port = listener.local_addr().unwrap().port();
    let invite = invite(&signer, NonZeroU16::new(port).unwrap());
    let code = encode(&invite).unwrap();
    let verifier = Arc::new(Verifier(invite.clone()));
    let request = JoinRequest {
        schema_version: JOIN_SCHEMA_VERSION,
        request_id: uuid::Uuid::new_v4(),
        invite_id: invite.claims().invite_id,
        peer: keys().0,
    };
    let reply = JoinReply {
        schema_version: JOIN_SCHEMA_VERSION,
        hub: signer.identity,
        revision: 2,
        applied: true,
    };
    let served = async {
        let (server, _) = listener.accept().await.unwrap();
        let yolo_hub_peer_transport::Accepted::Enrollment {
            connection,
            mut incoming,
        } = yolo_hub_peer_transport::accept_with_enrollment(
            server,
            transport,
            yolo_hub_peer_transport::PeerPins::new(vec![]).unwrap(),
            Some(verifier),
            deadline(),
        )
        .await
        .unwrap()
        else {
            panic!("expected an enrollment connection")
        };
        let yolo_hub_peer_transport::EnrollmentStream { mut stream, .. } =
            timeout_at(deadline(), incoming.recv()).await.unwrap().unwrap();
        assert_eq!(read_request(&mut stream).await.unwrap(), request);
        write_outcome(&mut stream, &JoinOutcome::Accepted { reply })
            .await
            .unwrap();
        use tokio::io::AsyncReadExt;
        let _ = timeout_at(deadline(), stream.read_u8()).await;
        let _ = connection.close(deadline()).await;
    };
    let joined = yolo_hub_peer_transport::join(
        "127.0.0.1",
        NonZeroU16::new(port).unwrap(),
        &code,
        &invite,
        &request,
        deadline(),
    );
    let (_, result) = tokio::join!(served, joined);
    assert_eq!(result.unwrap(), reply);
}

#[tokio::test]
async fn enrollment_rejects_a_wrong_code() {
    let (signer, transport) = hub();
    let listener = TcpListener::bind(("127.0.0.1", 0)).await.unwrap();
    let port = listener.local_addr().unwrap().port();
    let invite = invite(&signer, NonZeroU16::new(port).unwrap());
    let verifier = Arc::new(Verifier(invite.clone()));
    let request = JoinRequest {
        schema_version: JOIN_SCHEMA_VERSION,
        request_id: uuid::Uuid::new_v4(),
        invite_id: invite.claims().invite_id,
        peer: keys().0,
    };
    let served = async {
        let (server, _) = listener.accept().await.unwrap();
        yolo_hub_peer_transport::accept_with_enrollment(
            server,
            transport,
            yolo_hub_peer_transport::PeerPins::new(vec![]).unwrap(),
            Some(verifier),
            deadline(),
        )
        .await
    };
    let joined = yolo_hub_peer_transport::join(
        "127.0.0.1",
        NonZeroU16::new(port).unwrap(),
        "yolo1_wrong",
        &invite,
        &request,
        deadline(),
    );
    let (accepted, result) = tokio::join!(served, joined);
    assert!(accepted.is_err());
    assert_eq!(
        result.unwrap_err(),
        yolo_hub_peer_transport::JoinFailure::Authentication
    );
}
