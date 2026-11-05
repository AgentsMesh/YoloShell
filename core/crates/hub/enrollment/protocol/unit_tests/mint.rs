use super::*;
use yolo_hub_enrollment_protocol::{
    InviteError, InvitePolicy, InviteRequest, MintedInvite, SignedInvite, decode, encode, mint,
};

#[test]
fn policy_and_lifetime_are_bounded() {
    let signer = TestSigner::new();
    let closed = InvitePolicy {
        export_local: false,
        import_peer: false,
    };
    for (lifetime_ms, policy) in [
        (LIFETIME_MS, closed),
        (30_000, mutual()),
        (2 * 60 * 60 * 1000, mutual()),
    ] {
        assert!(matches!(
            SignedInvite::issue(claims(&signer, lifetime_ms, policy), &signer),
            Err(InviteError::Claims)
        ));
    }
}

#[test]
fn a_different_key_cannot_issue_an_invite() {
    let issuer = TestSigner::new();
    let other = TestSigner::new();
    assert!(matches!(
        SignedInvite::issue(claims(&issuer, LIFETIME_MS, mutual()), &other),
        Err(InviteError::Issuer)
    ));
}

#[test]
fn mint_binds_issuer_id_and_expiry() {
    let signer = TestSigner::new();
    let request = InviteRequest {
        dial: Some(InviteDial {
            host: HOST.into(),
            port: NonZeroU16::new(7400).unwrap(),
        }),
        lifetime_ms: LIFETIME_MS,
        policy: mutual(),
    };
    let invite_id = Uuid::new_v4();
    let invite = mint(&request, invite_id, NOW_MS, &signer).unwrap();
    let code = encode(&invite).unwrap();
    let minted = MintedInvite {
        invite_id,
        code,
        expires_at_ms: NOW_MS + LIFETIME_MS,
    };
    minted.validate().unwrap();
    let claims = invite.claims();
    assert_eq!(claims.issuer, signer.identity);
    assert_eq!(claims.invite_id, invite_id);
    assert_eq!(claims.dial, request.dial);
    assert_eq!(claims.policy, request.policy);
    assert!(matches!(
        MintedInvite {
            expires_at_ms: minted.expires_at_ms + 1,
            ..minted
        }
        .validate(),
        Err(InviteError::Code)
    ));
}

#[test]
fn an_address_free_request_mints_a_v2_code() {
    let signer = TestSigner::new();
    let request = InviteRequest {
        dial: None,
        lifetime_ms: LIFETIME_MS,
        policy: mutual(),
    };
    let invite = mint(&request, Uuid::new_v4(), NOW_MS, &signer).unwrap();
    assert_eq!(invite.claims().dial, None);
    let code = encode(&invite).unwrap();
    assert!(code.starts_with("yolo2_"), "{code}");
    let decoded = decode(&code).unwrap();
    assert_eq!(decoded.claims(), invite.claims());
    decoded.verify_at(NOW_MS).unwrap();
}

#[test]
fn invalid_mint_requests_are_rejected() {
    let signer = TestSigner::new();
    let request = |host: &str, lifetime_ms: u64, policy: InvitePolicy| InviteRequest {
        dial: Some(InviteDial {
            host: host.into(),
            port: NonZeroU16::new(7400).unwrap(),
        }),
        lifetime_ms,
        policy,
    };
    for invalid in [
        request("", LIFETIME_MS, mutual()),
        request("bad host", LIFETIME_MS, mutual()),
        request(HOST, 30_000, mutual()),
        request(HOST, 2 * 60 * 60 * 1000, mutual()),
        request(
            HOST,
            LIFETIME_MS,
            InvitePolicy {
                export_local: false,
                import_peer: false,
            },
        ),
    ] {
        assert!(matches!(
            mint(&invalid, Uuid::new_v4(), NOW_MS, &signer),
            Err(InviteError::Claims)
        ));
    }
}
