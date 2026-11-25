use crate::constants::ORDINAL_MULTIPLICITY_ERROR;
use crate::lcov::parse;
use crate::metrics::{Metric, Totals};
use crate::multiplicity_fixture::*;
use crate::scope_fixture::{self, CORE_SOURCE, SINGLE_INSTANCE};
use yolo_coverage_protocol::MetricKind;

#[test]
fn zero_intermediate_and_full_merge_hits_preserve_the_original_denominator() {
    for (report, multiplicity) in reports() {
        let complete = parse(&report).unwrap();
        assert!(complete.passes());
        for kind in MetricKind::ALL {
            for count in [u64::default(), SINGLE_REPORT_COUNT, multiplicity] {
                let totals = parse(&with_ordinal_count(&report, kind, count)).unwrap();
                let mut expected = complete;
                selected(&mut expected, kind).hit = u64::from(count != u64::default());
                assert_eq!(totals, expected);
                for (_, metric) in totals.metrics() {
                    assert_eq!(metric.found, SINGLE_REPORT_COUNT);
                }
            }
        }
    }
}

#[test]
fn each_metric_rejects_counts_exceeding_its_actual_merged_report_count() {
    for (report, multiplicity) in reports() {
        assert!(parse(&report).unwrap().passes());
        for kind in MetricKind::ALL {
            for count in [multiplicity + COUNT_STEP, u64::MAX] {
                let altered = with_ordinal_count(&report, kind, count);
                assert_eq!(parse(&altered).unwrap_err(), ORDINAL_MULTIPLICITY_ERROR);
            }
        }
    }
}

fn reports() -> [(String, u64); 2] {
    [
        (
            scope_fixture::report(CORE_SOURCE, SINGLE_INSTANCE, true),
            SINGLE_REPORT_COUNT,
        ),
        (merged(CORE_SOURCE), MERGED_REPORT_COUNT),
    ]
}

fn selected(totals: &mut Totals, kind: MetricKind) -> &mut Metric {
    match kind {
        MetricKind::Lines => &mut totals.lines,
        MetricKind::Functions => &mut totals.functions,
        MetricKind::Regions => &mut totals.regions,
        MetricKind::Branches => &mut totals.branches,
    }
}
