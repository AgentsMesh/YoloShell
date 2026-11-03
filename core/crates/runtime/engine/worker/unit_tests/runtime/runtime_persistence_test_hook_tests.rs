use super::*;

const AUDIT_FILE: &str = "runtime-persistence-audit.json";
const REMOVED_FILE: &str = "worker-launch.json";

#[test]
fn structured_audit_appends_publications_and_removals() {
    let directory = tempfile::tempdir().unwrap();
    let audit = directory.path().join(AUDIT_FILE);
    let removed = directory.path().join(REMOVED_FILE);
    record_at(
        &audit,
        RuntimePersistenceAuditEvent::Publication {
            bootstrap_delivery: BootstrapDelivery::Pending,
        },
    )
    .unwrap();
    record_at(
        &audit,
        RuntimePersistenceAuditEvent::Removal {
            path: removed.clone(),
        },
    )
    .unwrap();
    let events = read(&audit).unwrap();
    let mut events = events.iter();
    let publication = events.next().unwrap();
    let removal = events.next().unwrap();
    assert!(publication.publishes(BootstrapDelivery::Pending));
    assert!(!publication.publishes(BootstrapDelivery::Failed));
    assert!(removal.removes(&removed));
    assert!(events.next().is_none());
}
