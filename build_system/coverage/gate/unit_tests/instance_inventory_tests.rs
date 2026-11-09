use crate::instance_inventory::{InstanceInventories, validate};
use crate::lcov::ParsedReport;
use yolo_coverage_protocol::{MAXIMUM_MERGED_REPORT_COUNT, instance_inventory_digest};

const SOURCE: &str = "core/crates/worker/src/lib.rs";
const MEMBER: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
const OTHER_DIGEST: &str = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
const PRESENT: u64 = 1;
const DUPLICATED: u64 = 2;

fn report() -> ParsedReport {
    ParsedReport {
        production: [SOURCE.into()].into(),
        markers: [(SOURCE.into(), PRESENT)].into(),
        ..ParsedReport::default()
    }
}

fn assert_error(result: Result<(), String>, expected: &str) {
    assert_eq!(result.unwrap_err(), expected);
}

#[test]
fn inventory_header_and_members_reject_invalid_presence_counts() {
    for count in [u64::default(), MAXIMUM_MERGED_REPORT_COUNT + PRESENT] {
        let mut inventories = InstanceInventories::default();
        assert_error(
            inventories.record_header(SOURCE, OTHER_DIGEST, PRESENT, count),
            "instance inventory multiplicity is outside the protocol limit",
        );
        assert_error(
            inventories.record_member(SOURCE, OTHER_DIGEST, MEMBER, count),
            "instance inventory member multiplicity is outside the protocol limit",
        );
    }
}

#[test]
fn duplicate_headers_and_members_are_not_merged_as_additional_evidence() {
    let mut inventories = InstanceInventories::default();
    inventories
        .record_header(SOURCE, OTHER_DIGEST, PRESENT, PRESENT)
        .unwrap();
    assert_error(
        inventories.record_header(SOURCE, OTHER_DIGEST, PRESENT, PRESENT),
        "duplicate instance inventory header",
    );
    inventories
        .record_member(SOURCE, OTHER_DIGEST, MEMBER, PRESENT)
        .unwrap();
    assert_error(
        inventories.record_member(SOURCE, OTHER_DIGEST, MEMBER, PRESENT),
        "duplicate instance inventory member",
    );
}

#[test]
fn member_without_its_inventory_header_is_rejected() {
    let mut report = report();
    report
        .instance_inventories
        .record_member(SOURCE, OTHER_DIGEST, MEMBER, PRESENT)
        .unwrap();
    assert_error(validate(&report), "instance inventory has no header");
}

#[test]
fn inventory_outside_the_production_source_set_is_rejected() {
    let mut report = ParsedReport::default();
    report
        .instance_inventories
        .record_header(SOURCE, OTHER_DIGEST, PRESENT, PRESENT)
        .unwrap();
    assert_error(
        validate(&report),
        "instance inventory appears outside production sources",
    );
}

#[test]
fn header_count_cannot_hide_missing_or_extra_members() {
    for declared in [u64::default(), DUPLICATED] {
        let mut report = report();
        report
            .instance_inventories
            .record_header(SOURCE, OTHER_DIGEST, declared, PRESENT)
            .unwrap();
        report
            .instance_inventories
            .record_member(SOURCE, OTHER_DIGEST, MEMBER, PRESENT)
            .unwrap();
        assert_error(
            validate(&report),
            "instance inventory length differs from its header",
        );
    }
}

#[test]
fn member_set_must_match_the_committed_digest() {
    let mut report = report();
    report
        .instance_inventories
        .record_header(SOURCE, OTHER_DIGEST, PRESENT, PRESENT)
        .unwrap();
    report
        .instance_inventories
        .record_member(SOURCE, OTHER_DIGEST, MEMBER, PRESENT)
        .unwrap();
    assert_error(validate(&report), "instance inventory digest mismatch");
}

#[test]
fn correct_digest_does_not_authorize_a_different_member_multiplicity() {
    let mut report = report();
    let digest = instance_inventory_digest(&[MEMBER]).unwrap();
    report
        .instance_inventories
        .record_header(SOURCE, &digest, PRESENT, PRESENT)
        .unwrap();
    report
        .instance_inventories
        .record_member(SOURCE, &digest, MEMBER, DUPLICATED)
        .unwrap();
    assert_error(
        validate(&report),
        "instance inventory member multiplicity mismatch",
    );
}

#[test]
fn validated_empty_inventory_still_requires_matching_source_presence() {
    let digest = instance_inventory_digest(&[]).unwrap();
    let mut report = report();
    report
        .instance_inventories
        .record_header(SOURCE, &digest, u64::default(), PRESENT)
        .unwrap();
    assert!(validate(&report).is_ok());
    report.markers.insert(SOURCE.into(), DUPLICATED);
    assert_error(
        validate(&report),
        &format!("instance inventory multiplicity differs from marker in {SOURCE}"),
    );
}

#[test]
fn valid_member_digest_cannot_substitute_for_missing_metric_headers() {
    let digest = instance_inventory_digest(&[MEMBER]).unwrap();
    let mut report = report();
    report
        .instance_inventories
        .record_header(SOURCE, &digest, PRESENT, PRESENT)
        .unwrap();
    report
        .instance_inventories
        .record_member(SOURCE, &digest, MEMBER, PRESENT)
        .unwrap();
    assert_error(
        validate(&report),
        &format!("instance inventory differs from metric headers in {SOURCE}"),
    );
}
