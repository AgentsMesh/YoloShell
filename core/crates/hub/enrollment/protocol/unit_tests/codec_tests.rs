use super::*;
use yolo_hub_enrollment_protocol::{InviteError, SignedInvite, SignedInviteFields, decode, encode};

#[test]
fn signed_invites_round_trip_through_the_pairing_code() {
    let signer = TestSigner::new();
    let invite = signer.invite(LIFETIME_MS, mutual());
    let code = encode(&invite).unwrap();
    assert!(code.starts_with("yolo1_"));
    let decoded = decode(&code).unwrap();
    assert_eq!(decoded.claims(), invite.claims());
    decoded.verify_at(NOW_MS).unwrap();
}

#[test]
fn tampered_claims_or_signatures_are_rejected() {
    let signer = TestSigner::new();
    let invite = signer.invite(LIFETIME_MS, mutual());
    let mut fields = SignedInviteFields::from(invite);
    fields.claims.dial.host = "10.0.0.9".into();
    assert!(matches!(
        SignedInvite::try_from(fields),
        Err(InviteError::Signature)
    ));

    let mut fields = SignedInviteFields::from(signer.invite(LIFETIME_MS, mutual()));
    fields.signature[0] ^= 0xff;
    assert!(matches!(
        SignedInvite::try_from(fields),
        Err(InviteError::Signature)
    ));
}

#[test]
fn malformed_codes_are_rejected() {
    assert!(matches!(decode("yolo1_"), Err(InviteError::Code)));
    assert!(matches!(decode("not-a-code"), Err(InviteError::Code)));
    assert!(matches!(
        decode(&format!("yolo1_{}", "A".repeat(5000))),
        Err(InviteError::Code)
    ));
    let code = encode(&signer_invite()).unwrap();
    assert!(matches!(
        decode(&format!("{code}A")),
        Err(InviteError::Code)
    ));
    assert!(matches!(
        decode(&code[..code.len() - 4]),
        Err(InviteError::Code)
    ));
}

fn signer_invite() -> SignedInvite {
    TestSigner::new().invite(LIFETIME_MS, mutual())
}
