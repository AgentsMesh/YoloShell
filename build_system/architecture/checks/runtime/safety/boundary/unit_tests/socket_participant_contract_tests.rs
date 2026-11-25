use super::*;
use yolo_architecture_test_support::Fixture;

#[test]
fn participant_custody_contract_accepts_all_current_owners() {
    let fixture = Fixture::new();
    for (path, markers) in participant::CONTRACTS {
        fixture.write(path, &markers.join("\n"));
    }
    let violations = crate::inspect(fixture.root()).unwrap();
    assert!(
        !violations
            .iter()
            .any(|violation| violation.to_string().contains(PARTICIPANT_MESSAGE))
    );
}

#[test]
fn every_participant_lease_certificate_and_empty_cleanup_fence_is_required() {
    for (path, markers) in participant::CONTRACTS {
        for marker in *markers {
            let fixture = Fixture::new();
            let source = markers
                .join("\n")
                .replacen(marker, &format!("/* {marker} */"), 1);
            fixture.write(path, &source);
            let violations = crate::inspect(fixture.root()).unwrap();
            assert!(
                violations
                    .iter()
                    .any(|violation| violation.to_string().contains(PARTICIPANT_MESSAGE)),
                "{path}: {marker}"
            );
        }
    }
}
