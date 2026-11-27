use super::super::schema::OptionalParticipant;
use super::{rollover_fixture::RolloverFixture, validate};

#[test]
fn rollover_requires_the_domain_separated_source_proof() {
    let fixture = RolloverFixture::new();
    let mut intent = fixture.intent;
    custody_mut(&mut intent).source_object_sha256 = None;
    assert!(validate(&intent).is_err());
}

#[test]
fn payload_digest_cannot_replace_the_domain_separated_source_proof() {
    let fixture = RolloverFixture::new();
    let mut intent = fixture.intent;
    let custody = custody_mut(&mut intent);
    custody.source_object_sha256 = Some(custody.source_sha256);
    assert!(validate(&intent).is_err());
}

fn custody_mut(
    intent: &mut super::super::RebootMigrationIntent,
) -> &mut super::super::schema::RequiredParticipant<crate::pty_custody::CustodyDurableRecord> {
    let OptionalParticipant::Replace(custody) = &mut intent.pty_custody else {
        unreachable!();
    };
    custody
}
