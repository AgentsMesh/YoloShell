use super::*;
use yolo_hub_enrollment_protocol::InviteError;

#[test]
fn expired_and_not_yet_valid_invites_are_rejected() {
    let signer = TestSigner::new();
    let invite = signer.invite(LIFETIME_MS, mutual());
    assert!(matches!(
        invite.verify_at(NOW_MS + LIFETIME_MS),
        Err(InviteError::Expired)
    ));
    assert!(matches!(
        invite.verify_at(NOW_MS - 61_000),
        Err(InviteError::Expired)
    ));
    invite.verify_at(NOW_MS - 60_000).unwrap();
}
