use crate::lcov::parse;
use yolo_coverage_protocol::{MARKER_TOKEN, MINIMUM_SOURCE_POSITION, source_inventory_token};

const SOURCE: &str = "core/src/worker.rs";
const INVENTORY: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";

#[test]
fn synthetic_fnda_without_definition_fails_closed() {
    let report =
        format!("SF:{SOURCE}\nFNDA:1,{MARKER_TOKEN}\nFNF:0\nFNH:0\nLF:0\nLH:0\nend_of_record\n");
    assert!(parse(&report).is_err());
}

#[test]
fn raw_zero_hit_mapping_is_not_an_empty_placeholder() {
    let report = format!("SF:{SOURCE}\nDA:1,0\nLF:1\nLH:0\nFNF:0\nFNH:0\nend_of_record\n");
    assert!(parse(&report).is_err());
}

#[test]
fn duplicate_empty_summary_is_not_an_empty_placeholder() {
    let report = format!("SF:{SOURCE}\nFNF:0\nFNF:0\nFNH:0\nLF:0\nLH:0\nend_of_record\n");
    assert!(parse(&report).is_err());
}

#[test]
fn inventory_without_source_marker_fails_closed() {
    let token = source_inventory_token(INVENTORY, 1);
    let report = format!(
        "SF:{SOURCE}\nFN:{MINIMUM_SOURCE_POSITION},{token}\nFNDA:1,{token}\nFNF:1\nFNH:1\nLF:0\nLH:0\nend_of_record\n"
    );
    assert!(parse(&report).is_err());
}

#[test]
fn duplicate_counts_cannot_select_which_presence_evidence_wins() {
    let report = format!(
        "SF:{SOURCE}\nFN:{MINIMUM_SOURCE_POSITION},{MARKER_TOKEN}\nFNDA:0,{MARKER_TOKEN}\nFNDA:1,{MARKER_TOKEN}\nend_of_record\n"
    );
    assert!(
        parse(&report)
            .unwrap_err()
            .contains("duplicate synthetic FNDA entry")
    );
}
