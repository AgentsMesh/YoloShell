use super::{constants::*, fixtures::*};
use crate::authority::{Authority, retire_expired};
use std::time::Duration;
use tokio::time::Instant;

#[test]
fn grant_lifetime_includes_network_latency_and_expiration_has_no_grace_period() {
    let sent = Instant::now();
    let expiry = sent + Duration::from_millis(TTL_MS);
    let grant = grant();
    let authority = Authority::received(grant, sent, sent).unwrap();
    assert_eq!(authority.grant, grant);
    assert_eq!(authority.expires_at, expiry);
    assert_eq!(authority.renew_at, sent + Duration::from_millis(RENEW_MS));
    assert!(authority.is_current(sent));
    assert!(!authority.is_current(expiry));
    assert!(Authority::received(grant, sent, expiry).is_none());
    assert!(Authority::received(grant, sent, expiry + Duration::from_millis(RENEW_MS)).is_none());
}

#[test]
fn delayed_snapshot_readiness_cannot_republish_expired_writer_authority() {
    let sent = Instant::now();
    let mut authority = Authority::received(grant(), sent, sent);
    assert!(!retire_expired(&mut authority, sent));
    assert!(authority.is_some());
    assert!(retire_expired(
        &mut authority,
        sent + Duration::from_millis(TTL_MS)
    ));
    assert!(authority.is_none());
    assert!(!retire_expired(
        &mut authority,
        sent + Duration::from_millis(TTL_MS)
    ));
}
