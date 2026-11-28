use super::*;
use serde_json::{Value, json};

#[test]
fn start_rejects_a_tampered_profile_with_the_previous_digest() {
    let mut document = committed();
    document["profile"]["displayName"] = json!("Tampered host");
    let fixture = Fixture::new(document);
    assert!(
        StartAuthorityGuard::acquire(&fixture.document, &fixture.lock, PROFILE_SHA256).is_err()
    );
}

#[test]
fn blocked_previous_profile_requires_its_exact_digest() {
    let mutation_id = Uuid::new_v4();
    let mut document = blocked(mutation_id);
    document["previousProfile"] = full_profile();
    document["previousProfileSHA256"] = json!(PROFILE_SHA256);
    let fixture = Fixture::new(document.clone());
    RetirementAuthorityGuard::acquire(&fixture.document, &fixture.lock, HOST_ID, mutation_id)
        .unwrap();

    document["previousProfile"]["displayName"] = json!("Tampered host");
    let fixture = Fixture::new(document);
    assert!(
        RetirementAuthorityGuard::acquire(&fixture.document, &fixture.lock, HOST_ID, mutation_id)
            .is_err()
    );
}

#[test]
fn authority_document_rejects_unknown_and_inconsistent_fields() {
    let mut unknown = committed();
    unknown["unexpected"] = json!(true);
    let fixture = Fixture::new(unknown);
    assert!(
        StartAuthorityGuard::acquire(&fixture.document, &fixture.lock, PROFILE_SHA256).is_err()
    );

    let fixture = Fixture::new(json!({
        "version": 1,
        "hostID": HOST_ID,
        "phase": "deleted",
        "profileSHA256": PROFILE_SHA256,
    }));
    assert!(
        StartAuthorityGuard::acquire(&fixture.document, &fixture.lock, PROFILE_SHA256).is_err()
    );

    let mut explicit_null = committed();
    explicit_null["mutationID"] = Value::Null;
    let fixture = Fixture::new(explicit_null);
    assert!(
        StartAuthorityGuard::acquire(&fixture.document, &fixture.lock, PROFILE_SHA256).is_err()
    );
}

#[test]
fn blocked_authority_rejects_an_orphaned_stale_start() {
    let fixture = Fixture::new(blocked(Uuid::new_v4()));
    assert!(
        StartAuthorityGuard::acquire(&fixture.document, &fixture.lock, PROFILE_SHA256).is_err()
    );
}

#[test]
fn late_retirement_rejects_after_a_new_profile_commits() {
    let mutation_id = Uuid::new_v4();
    let fixture = Fixture::new(blocked(mutation_id));
    write_private(
        &fixture.document,
        &serde_json::to_vec(&committed()).unwrap(),
    );
    assert!(
        RetirementAuthorityGuard::acquire(&fixture.document, &fixture.lock, HOST_ID, mutation_id)
            .is_err()
    );
}

#[test]
fn guard_rejects_same_content_document_namespace_replacement() {
    let fixture = Fixture::new(committed());
    let guard =
        StartAuthorityGuard::acquire(&fixture.document, &fixture.lock, PROFILE_SHA256).unwrap();
    fs::rename(
        &fixture.document,
        fixture.document.with_extension("displaced"),
    )
    .unwrap();
    write_private(
        &fixture.document,
        &serde_json::to_vec(&committed()).unwrap(),
    );
    assert!(guard.verify().is_err());
}
