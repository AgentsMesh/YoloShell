use crate::lcov::parse;
use crate::lcov_fixture::{Decorations, Entry};
use crate::metrics::{Metric, Totals};
use yolo_coverage_protocol::{MARKER_TOKEN, MetricKind, header_token, ordinal_token};

#[path = "lcov_boundary_tests.rs"]
mod boundary_tests;

const SOURCE: &str = "core/src/worker.rs";
const ID_A: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
const ID_B: &str = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
const SIGNATURE: &str = "cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc";
const GROUP_LINE: u64 = 7;
const GROUP_COLUMN: u64 = 2;
const EMPTY_PRODUCTION_RECORD: &str =
    "SF:core/src/paths/constants.rs\nFNF:0\nFNH:0\nLF:0\nLH:0\nend_of_record\n";
const MALFORMED_EMPTY_RECORDS: [&str; 3] = [
    "SF:core/src/paths/constants.rs\nend_of_record\n",
    "SF:core/src/paths/constants.rs\nFNF:0\nFNH:1\nLF:0\nLH:1\nend_of_record\n",
    "SF:core/src/paths/constants.rs\nFNF:0\nFNH:0\nLF:0\nLH:0\nunknown:0\nend_of_record\n",
];

#[test]
fn generic_instances_use_independent_group_maxima() {
    let entries = [
        instance(ID_A, SIGNATURE, &[1, 0], &[1], &[1, 0], &[1, 0], 2),
        instance(ID_B, SIGNATURE, &[0, 1], &[1], &[0, 1], &[0, 1], 2),
    ]
    .concat();
    assert_eq!(
        parse(&report(&entries, true)).unwrap(),
        Totals {
            lines: Metric { found: 2, hit: 1 },
            functions: Metric { found: 1, hit: 1 },
            regions: Metric { found: 2, hit: 1 },
            branches: Metric { found: 2, hit: 1 },
        }
    );
}

#[test]
fn one_instance_merged_across_tests_preserves_complementary_hits() {
    let first = instance(ID_A, SIGNATURE, &[1, 0], &[1], &[1, 0], &[1, 0], 2);
    let second = instance(ID_A, SIGNATURE, &[0, 1], &[1], &[0, 1], &[0, 1], 2);
    let merged = crate::multiplicity_fixture::merge(&report(&first, true), &report(&second, true));
    let totals = parse(&merged).unwrap();
    assert_eq!(totals.lines, Metric { found: 2, hit: 2 });
    assert_eq!(totals.regions, Metric { found: 2, hit: 2 });
    assert_eq!(totals.branches, Metric { found: 2, hit: 2 });
}

#[test]
fn raw_lcov_details_cannot_change_exact_metrics() {
    let entries = instance(ID_A, SIGNATURE, &[1, 0], &[1], &[1, 0], &[1, 0], 2);
    let totals = parse(&report(&entries, true)).unwrap();
    assert_eq!(totals.lines.hit, 1);
    assert_eq!(totals.functions.found, 1);
    assert_eq!(totals.branches.hit, 1);
}

#[test]
fn every_production_source_requires_exactly_one_marker() {
    let entries = instance(ID_A, SIGNATURE, &[1], &[1], &[1], &[1], 1);
    assert!(parse(&report(&entries, false)).is_err());
    let duplicate_marker = [entries, vec![(1, MARKER_TOKEN.to_owned(), 1)]].concat();
    assert!(parse(&report(&duplicate_marker, true)).is_err());
}

#[test]
fn empty_uninstrumentable_production_records_do_not_require_markers() {
    let entries = instance(ID_A, SIGNATURE, &[1], &[1], &[1], &[1], 1);
    let baseline = report(&entries, true);
    let combined = format!("{baseline}{EMPTY_PRODUCTION_RECORD}");
    assert_eq!(parse(&combined).unwrap(), parse(&baseline).unwrap());
}

#[test]
fn malformed_empty_records_cannot_bypass_source_markers() {
    let entries = instance(ID_A, SIGNATURE, &[1], &[1], &[1], &[1], 1);
    let baseline = report(&entries, true);
    for malformed in MALFORMED_EMPTY_RECORDS {
        assert!(parse(&format!("{baseline}{malformed}")).is_err());
    }
}

#[test]
fn source_and_instance_inventories_are_required() {
    let entries = instance(ID_A, SIGNATURE, &[1], &[1], &[1], &[1], 1);
    for decorations in [
        Decorations::MarkerOnly,
        Decorations::MarkerAndSourceInventory,
    ] {
        let report = crate::lcov_fixture::report(SOURCE, &entries, &entries, decorations);
        assert!(parse(&report).is_err());
    }
}

#[test]
fn missing_duplicate_and_incompatible_instances_fail_closed() {
    let complete = instance(ID_A, SIGNATURE, &[1], &[1], &[1], &[1], 1);
    let missing = complete
        .iter()
        .filter(|(_, name, _)| !name.contains("/header/regions/"))
        .cloned()
        .collect::<Vec<_>>();
    assert!(parse(&report_with_inventory(&missing, &complete, true)).is_err());
    let duplicate = [complete.clone(), vec![complete[0].clone()]].concat();
    assert!(parse(&report(&duplicate, true)).is_err());
    let incompatible = [
        complete.clone(),
        metric(MetricKind::Lines, ID_A, ID_B, &[1], None),
    ]
    .concat();
    assert!(parse(&report_with_inventory(&incompatible, &complete, true)).is_err());
}

#[test]
fn malformed_tokens_lengths_and_zero_global_denominators_fail_closed() {
    let malformed_entries = [(GROUP_LINE, "__yolo_metric__/unknown".to_owned(), 1)];
    let inventory = instance(ID_A, SIGNATURE, &[1], &[1], &[1], &[1], 1);
    let malformed = report_with_inventory(&malformed_entries, &inventory, true);
    assert!(parse(&malformed).is_err());
    let zero_branch = instance(ID_A, SIGNATURE, &[1], &[1], &[1], &[], 0);
    assert!(parse(&report(&zero_branch, true)).is_err());
}

fn report(entries: &[Entry], marker: bool) -> String {
    report_with_inventory(entries, entries, marker)
}

fn report_with_inventory(entries: &[Entry], inventory: &[Entry], marker: bool) -> String {
    let decorations = if marker {
        Decorations::Complete
    } else {
        Decorations::None
    };
    crate::lcov_fixture::report(SOURCE, entries, inventory, decorations)
}

fn instance(
    identity: &str,
    signature: &str,
    lines: &[u64],
    functions: &[u64],
    regions: &[u64],
    branches: &[u64],
    branch_found: u64,
) -> Vec<Entry> {
    [
        metric(MetricKind::Lines, identity, signature, lines, None),
        metric(MetricKind::Functions, identity, signature, functions, None),
        metric(MetricKind::Regions, identity, signature, regions, None),
        metric(
            MetricKind::Branches,
            identity,
            signature,
            branches,
            Some(branch_found),
        ),
    ]
    .concat()
}

fn metric(
    kind: MetricKind,
    identity: &str,
    signature: &str,
    counts: &[u64],
    found: Option<u64>,
) -> Vec<Entry> {
    let length = counts.len() as u64;
    let header = header_token(
        kind,
        GROUP_LINE,
        GROUP_COLUMN,
        identity,
        signature,
        length,
        found.unwrap_or(length),
    );
    let mut entries = vec![(GROUP_LINE, header, 1)];
    entries.extend(counts.iter().copied().enumerate().map(|(ordinal, count)| {
        (
            GROUP_LINE,
            ordinal_token(
                kind,
                GROUP_LINE,
                GROUP_COLUMN,
                identity,
                signature,
                ordinal as u64,
            ),
            count,
        )
    }));
    entries
}
