use crate::{fixtures::*, support::*};
use yolo_hub_peer_protocol::*;

#[test]
fn catalogue_requires_origin_signature_and_the_immediate_relay_delegation() {
    let fixture = Fixture::new();
    fixture
        .export
        .verify(
            fixture.entry.identity,
            fixture.relay.identity,
            fixture.origin.identity,
            NOW,
        )
        .unwrap();
    assert!(
        fixture
            .export
            .verify(
                fixture.entry.identity,
                fixture.origin.identity,
                fixture.origin.identity,
                NOW
            )
            .is_err()
    );
    let mut forged = fixture.export.clone();
    forged.catalogue.payload.terminals.clear();
    assert!(
        forged
            .verify(
                fixture.entry.identity,
                fixture.relay.identity,
                fixture.origin.identity,
                NOW
            )
            .is_err()
    );
    let mut forged = fixture.export.clone();
    forged.catalogue.purpose = Purpose::Response;
    assert!(
        forged
            .verify(
                fixture.entry.identity,
                fixture.relay.identity,
                fixture.origin.identity,
                NOW
            )
            .is_err()
    );
    let mut forged = fixture.export.clone();
    forged.catalogue.signature.pop();
    assert!(
        forged
            .verify(
                fixture.entry.identity,
                fixture.relay.identity,
                fixture.origin.identity,
                NOW
            )
            .is_err()
    );
    assert!(
        fixture
            .export
            .verify(
                fixture.entry.identity,
                fixture.relay.identity,
                fixture.entry.identity,
                NOW
            )
            .is_err()
    );
}

#[test]
fn catalogue_rejects_duplicate_or_foreign_resources_and_invalid_lifetimes() {
    let fixture = Fixture::new();
    let original = fixture.export.catalogue.payload;
    original.validate(fixture.origin.identity).unwrap();
    let mut changed = original.clone();
    changed.terminals.push(changed.terminals[0].clone());
    assert!(changed.validate(fixture.origin.identity).is_err());
    let mut changed = original.clone();
    changed.terminals = vec![session(fixture.entry.identity)];
    assert!(changed.validate(fixture.origin.identity).is_err());
    let mut changed = original.clone();
    changed.expires_at_ms = changed.issued_at_ms;
    assert!(changed.validate(fixture.origin.identity).is_err());
    assert!(original.is_current(NOW));
    assert!(!original.is_current(EXPIRES));
}

#[test]
fn documents_roundtrip_and_do_not_confuse_signature_purposes() {
    let fixture = Fixture::new();
    let encoded = serde_json::to_vec(&fixture.export).unwrap();
    let decoded: ExportedOrigin = serde_json::from_slice(&encoded).unwrap();
    assert_eq!(decoded, fixture.export);
    let call = fixture.call();
    assert!(call.verify(Purpose::Call, fixture.entry.identity).is_ok());
    assert!(
        call.verify(Purpose::Catalogue, fixture.entry.identity)
            .is_err()
    );
    let mut changed = call.clone();
    changed.payload.expires_at_ms += STEP;
    assert_ne!(changed.fingerprint().unwrap(), call.fingerprint().unwrap());
}
