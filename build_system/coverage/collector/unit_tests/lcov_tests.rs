use crate::lcov::augment;
use crate::model::{Instance, InstanceMap};
use std::collections::{BTreeMap, BTreeSet};
use yolo_coverage_protocol::{MARKER_TOKEN, MetricKind, ordinal_token};

#[path = "lcov_raw_function_tests.rs"]
mod raw_function_tests;

const SOURCE: &str = "core/src/lib.rs";
const TOOLING_SOURCE: &str = "build_system/tool/src/lib.rs";
const DIGEST: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
const GROUP_LINE: u64 = 1;
const GROUP_COLUMN: u64 = 1;
const EXPECTED_PRESENCE_COUNT: u64 = 1;

#[test]
fn raw_records_markers_and_every_instance_metric_are_emitted() {
    let input = format!("SF:{SOURCE}\nLF:1\nLH:1\nend_of_record\n");
    let output = augment(&input, &instances(), &sources()).unwrap();
    assert!(output.starts_with(&format!("SF:{SOURCE}\n")));
    assert!(output.contains(&format!("FN:1,{MARKER_TOKEN}")));
    for metric in [
        "/header/lines/",
        "/header/functions/",
        "/header/regions/",
        "/header/branches/",
    ] {
        assert!(output.contains(metric), "{metric}");
    }
    assert!(output.contains("FNDA:1,__yolo_metric__/ordinal/lines/"));
    assert!(output.contains("FNDA:0,__yolo_metric__/ordinal/lines/"));
}

#[test]
fn execution_counts_are_encoded_as_presence_before_bazel_merge() {
    let input = format!("SF:{SOURCE}\nLF:1\nLH:1\nend_of_record\n");
    let output = augment(&input, &instances(), &sources()).unwrap();
    for (ordinal, expected) in [
        (u64::default(), EXPECTED_PRESENCE_COUNT),
        (EXPECTED_PRESENCE_COUNT, u64::default()),
    ] {
        let token = ordinal_token(
            MetricKind::Lines,
            GROUP_LINE,
            GROUP_COLUMN,
            DIGEST,
            DIGEST,
            ordinal,
        );
        assert!(output.contains(&format!("FNDA:{expected},{token}")));
    }
    assert!(!output.contains(&format!("FNDA:{},__yolo_metric__/", u64::MAX)));
}

#[test]
fn production_sources_must_match_json_summaries() {
    let input = format!("SF:{SOURCE}\nend_of_record\n");
    assert!(augment(&input, &instances(), &BTreeSet::new()).is_err());
    assert!(
        augment(
            "SF:core/src/other.rs\nend_of_record\n",
            &instances(),
            &sources()
        )
        .is_err()
    );
}

#[test]
fn malformed_boundaries_and_duplicate_sources_fail_closed() {
    for report in [
        "SF:\nend_of_record\n",
        "end_of_record\n",
        "SF:core/src/a.rs\nSF:core/src/b.rs\n",
        "SF:core/src/a.rs\n",
        "SF:core/src/lib.rs\nend_of_record\nSF:core/src/lib.rs\nend_of_record\n",
    ] {
        assert!(
            augment(report, &instances(), &sources()).is_err(),
            "{report:?}"
        );
    }
}

#[test]
fn nonproduction_records_are_preserved_without_synthetic_data() {
    let input = "TN:yolo\nSF:integration_tests/runtime/protocol.rs\nLF:1\nLH:1\nend_of_record\n";
    assert_eq!(
        augment(input, &InstanceMap::new(), &BTreeSet::new()).unwrap(),
        input
    );
}

#[test]
fn tooling_sources_share_complete_inventory_but_test_sources_get_no_markers() {
    let mut instances = instances();
    let mut tooling = instances[SOURCE].clone();
    for instance in &mut tooling {
        instance.source = TOOLING_SOURCE.to_owned();
    }
    instances.insert(TOOLING_SOURCE.to_owned(), tooling);
    let sources = BTreeSet::from([SOURCE.to_owned(), TOOLING_SOURCE.to_owned()]);
    let ignored = "SF:build_system/tool/unit_tests/case.rs\nLF:1\nLH:1\nend_of_record\n";
    let input =
        format!("SF:{SOURCE}\nend_of_record\nSF:{TOOLING_SOURCE}\nend_of_record\n{ignored}");
    let output = augment(&input, &instances, &sources).unwrap();
    assert_eq!(
        output.matches(&format!("FN:1,{MARKER_TOKEN}")).count(),
        sources.len()
    );
    assert!(output.ends_with(ignored));
    assert!(augment(&input, &instances, &BTreeSet::from([SOURCE.to_owned()])).is_err());
}

fn sources() -> BTreeSet<String> {
    BTreeSet::from([SOURCE.to_owned()])
}

fn instances() -> InstanceMap {
    let instance = Instance {
        branch_found: 1,
        column: GROUP_COLUMN,
        identity: DIGEST.to_owned(),
        line: GROUP_LINE,
        name: "function".to_owned(),
        signature: DIGEST.to_owned(),
        source: SOURCE.to_owned(),
        vectors: BTreeMap::from([
            (MetricKind::Lines, vec![u64::MAX, 0]),
            (MetricKind::Functions, vec![1]),
            (MetricKind::Regions, vec![1]),
            (MetricKind::Branches, vec![1, 0]),
        ]),
    };
    BTreeMap::from([(SOURCE.to_owned(), vec![instance])])
}
