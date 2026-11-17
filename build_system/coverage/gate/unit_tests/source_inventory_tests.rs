use crate::lcov::ParsedReport;
use crate::scope_fixture::{CORE_SOURCE, SHARED_SOURCES, TOOLING_SOURCE};
use crate::source_inventory::{SourceInventories, validate};
use yolo_coverage_protocol::{MAXIMUM_MERGED_REPORT_COUNT, source_inventory_digest};

const PRESENT: u64 = 1;
const TWO_MEMBERS: u64 = 2;
const OTHER_DIGEST: &str = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";

fn report(sources: &[&str]) -> ParsedReport {
    ParsedReport {
        production: sources.iter().map(|source| (*source).into()).collect(),
        markers: sources
            .iter()
            .map(|source| ((*source).into(), PRESENT))
            .collect(),
        ..ParsedReport::default()
    }
}

#[test]
fn source_inventory_rejects_conflicting_member_counts_and_duplicate_locations() {
    let mut inventories = SourceInventories::default();
    inventories
        .record(OTHER_DIGEST, PRESENT, CORE_SOURCE, PRESENT)
        .unwrap();
    assert_eq!(
        inventories
            .record(OTHER_DIGEST, TWO_MEMBERS, TOOLING_SOURCE, PRESENT)
            .unwrap_err(),
        "source inventory member counts disagree"
    );
    assert_eq!(
        inventories
            .record(OTHER_DIGEST, PRESENT, CORE_SOURCE, PRESENT)
            .unwrap_err(),
        "duplicate source inventory token"
    );
}

#[test]
fn source_inventory_rejects_absent_or_overflowing_presence_evidence() {
    for count in [u64::default(), MAXIMUM_MERGED_REPORT_COUNT + PRESENT] {
        assert_eq!(
            SourceInventories::default()
                .record(OTHER_DIGEST, PRESENT, CORE_SOURCE, count)
                .unwrap_err(),
            "source inventory multiplicity is outside the protocol limit"
        );
    }
}

#[test]
fn complete_member_set_must_match_its_committed_digest() {
    let mut report = report(&[CORE_SOURCE]);
    report
        .source_inventories
        .record(OTHER_DIGEST, PRESENT, CORE_SOURCE, PRESENT)
        .unwrap();
    assert_eq!(
        validate(&report).unwrap_err(),
        "source inventory digest mismatch"
    );
}

#[test]
fn authenticated_inventory_cannot_introduce_an_unmarked_production_source() {
    let mut report = report(&[CORE_SOURCE]);
    let digest = source_inventory_digest(&SHARED_SOURCES).unwrap();
    for source in SHARED_SOURCES {
        report
            .source_inventories
            .record(&digest, TWO_MEMBERS, source, PRESENT)
            .unwrap();
    }
    assert_eq!(
        validate(&report).unwrap_err(),
        "source inventory membership is inconsistent"
    );
}

#[test]
fn inventory_members_must_have_equal_merge_multiplicities() {
    let mut report = report(&SHARED_SOURCES);
    let digest = source_inventory_digest(&SHARED_SOURCES).unwrap();
    report
        .source_inventories
        .record(&digest, TWO_MEMBERS, CORE_SOURCE, PRESENT)
        .unwrap();
    report
        .source_inventories
        .record(&digest, TWO_MEMBERS, TOOLING_SOURCE, TWO_MEMBERS)
        .unwrap();
    assert_eq!(
        validate(&report).unwrap_err(),
        "source inventory membership is inconsistent"
    );
}

#[test]
fn shared_inventory_authorizes_only_the_exact_marked_sources() {
    let mut report = report(&SHARED_SOURCES);
    let digest = source_inventory_digest(&SHARED_SOURCES).unwrap();
    for source in SHARED_SOURCES {
        report
            .source_inventories
            .record(&digest, TWO_MEMBERS, source, PRESENT)
            .unwrap();
    }
    validate(&report).unwrap();
    report.markers.insert(CORE_SOURCE.into(), TWO_MEMBERS);
    assert_eq!(
        validate(&report).unwrap_err(),
        format!("source inventory multiplicity differs from marker in {CORE_SOURCE}")
    );
}
