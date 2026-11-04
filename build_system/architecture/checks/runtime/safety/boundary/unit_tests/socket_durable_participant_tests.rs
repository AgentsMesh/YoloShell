use super::*;
use yolo_architecture_test_support::Fixture;

fn current(path: &str) -> String {
    participant::CONTRACTS
        .iter()
        .find(|(owner, _)| *owner == path)
        .unwrap()
        .1
        .join("\n")
}

fn rejected(path: &str, text: &str) -> bool {
    let fixture = Fixture::new();
    fixture.write(path, text);
    crate::inspect(fixture.root())
        .unwrap()
        .iter()
        .any(|violation| violation.to_string().contains(PARTICIPANT_MESSAGE))
}

#[test]
fn durable_certificate_cannot_revert_to_live_ids_or_discard_retained_live_fences() {
    for (path, before, after) in [
        (
            participant::CERTIFICATE,
            "CERTIFICATE_SCHEMA_VERSION: u16 = 3",
            "CERTIFICATE_SCHEMA_VERSION: u16 = 2",
        ),
        (
            participant::CERTIFICATE_RECORD,
            "directory: DurableObjectIdentity",
            "directory: ObjectIdentity",
        ),
        (
            participant::CERTIFICATE_RECORD,
            "registration: DurableObjectIdentity",
            "registration: ObjectIdentity",
        ),
        (
            participant::CERTIFICATE_IO,
            "directory.durable_identity()?",
            "directory.object_identity()?",
        ),
        (
            participant::CERTIFICATE_IO,
            "Some(expected)",
            "Some(unverified)",
        ),
        (
            participant::AUTHORITY,
            "self.registration.verify()?",
            "Ok(())?",
        ),
        (
            participant::AUTHORITY,
            "self.registration.durable_identity()",
            "foreign.durable_identity()",
        ),
        (
            participant::STAGE,
            "self.directory.object_identity()? == self.live_identity",
            "true",
        ),
        (
            participant::REGISTRATION_READ,
            "opened.durable_version(entry.path(), REGISTRATION_MAX_BYTES)?",
            "foreign.durable_version(entry.path(), REGISTRATION_MAX_BYTES)?",
        ),
        (
            participant::REGISTRATION,
            "verify_current_unchanged",
            "verify_current_content_version",
        ),
    ] {
        let source = current(path);
        assert!(source.contains(before));
        assert!(
            rejected(path, &source.replace(before, after)),
            "{path}: {before}"
        );
    }
}

#[test]
fn work_bounded_orphan_cleanup_retains_both_fences_even_with_a_test_observer() {
    let source = current(participant::BOOKKEEPING).replace(
        "let result = scanner.scan_chunk()?;",
        "let result = scanner.scan_chunk()?; #[cfg(test)] observe_completed_chunk();",
    );
    assert!(!rejected(participant::BOOKKEEPING, &source));
    for (before, after) in [
        ("fence()?; let result", "let result"),
        ("fence()?; match result", "match result"),
        ("0..SOCKET_STAGE_RETIRED_CHUNK_LIMIT", "0..usize::MAX"),
        ("if !only_bookkeeping(directory)? { return Ok(false); }", ""),
    ] {
        assert!(source.contains(before));
        assert!(
            rejected(participant::BOOKKEEPING, &source.replace(before, after)),
            "{before}"
        );
    }
    let timed = format!(
        "{source}\nlet deadline = {};",
        participant::REUSED_LEASE_DEADLINE
    );
    assert!(rejected(participant::BOOKKEEPING, &timed));
}
