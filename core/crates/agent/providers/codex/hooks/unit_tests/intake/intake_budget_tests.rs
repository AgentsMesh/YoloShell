use super::intake_test_support::*;

const OVERFLOW_ENTRY_COUNT: usize = 1;

#[test]
fn directory_scan_has_a_strict_entry_budget() {
    let fixture = IntakeFixture::new();
    let intake = fixture.intake();
    fill_unknown_entries(&intake);
    assert!(intake.scan().is_ok());
    std::fs::write(
        intake.path(&format!(
            "{UNKNOWN_ENTRY_PREFIX}{}",
            super::super::intake_constants::MAX_DIRECTORY_ENTRIES
                .saturating_add(OVERFLOW_ENTRY_COUNT)
        )),
        [],
    )
    .unwrap();
    assert!(intake.scan().is_err());
}

#[test]
fn persistence_rejects_a_full_directory_without_leaking_a_temporary_file() {
    let fixture = IntakeFixture::new();
    let intake = fixture.intake();
    fill_unknown_entries(&intake);

    assert!(intake.persist_occurrence(fixture.envelope()).is_err());

    assert!(intake.scan().is_ok());
    let entries = std::fs::read_dir(intake.directory().path())
        .unwrap()
        .count();
    assert_eq!(
        entries,
        super::super::intake_constants::MAX_DIRECTORY_ENTRIES
    );
}

#[test]
fn pending_budget_preserves_space_to_quarantine_every_occurrence() {
    let fixture = IntakeFixture::new();
    let intake = fixture.intake();
    for _ in 0..super::super::intake_constants::MAX_PENDING_OCCURRENCES {
        intake.persist_occurrence(fixture.envelope()).unwrap();
    }
    assert!(intake.persist_occurrence(fixture.envelope()).is_err());

    let scan = intake.scan().unwrap();
    assert_eq!(
        scan.occurrences.len(),
        super::super::intake_constants::MAX_PENDING_OCCURRENCES
    );
    for occurrence in &scan.occurrences {
        intake.quarantine(occurrence).unwrap();
    }

    let quarantined = intake.scan().unwrap();
    assert!(quarantined.occurrences.is_empty());
    assert_eq!(
        quarantined.quarantined,
        super::super::intake_constants::MAX_PENDING_OCCURRENCES
    );
    assert_eq!(
        std::fs::read_dir(intake.directory().path())
            .unwrap()
            .count(),
        super::super::intake_constants::MAX_DIRECTORY_ENTRIES
    );
}

fn fill_unknown_entries(intake: &crate::codex_hooks::CodexHookIntake) {
    for index in 0..super::super::intake_constants::MAX_DIRECTORY_ENTRIES {
        std::fs::write(intake.path(&format!("{UNKNOWN_ENTRY_PREFIX}{index}")), []).unwrap();
    }
}
