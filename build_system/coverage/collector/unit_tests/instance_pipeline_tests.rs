use crate::function_report::{FunctionMetrics, FunctionReport};
use crate::instance_fixture::*;
use crate::{instances, validation};
use std::path::Path;
use yolo_coverage_protocol::MetricKind;

#[test]
fn parsed_instances_reconcile_every_metric_with_file_summaries() {
    let parsed = instances::parse(&data(vec![function()]), report(), Path::new(ROOT)).unwrap();
    let instance = &parsed[SOURCE][FILE_INDEX as usize];
    assert_eq!(instance.source, SOURCE);
    assert_eq!(instance.name, FUNCTION);
    assert_eq!(instance.branch_found, BRANCH_OUTCOMES);
    for kind in MetricKind::ALL {
        assert!(instance.metric(kind).unwrap().hit > u64::default());
    }
    validation::validate(&summaries(), &parsed).unwrap();
}

#[test]
fn missing_extra_and_duplicate_function_rows_fail_closed() {
    let input = data(vec![function()]);
    assert!(instances::parse(&input, FunctionReport::new(), Path::new(ROOT)).is_err());
    let mut report = report();
    let metrics = report[&(SOURCE.to_owned(), FUNCTION.to_owned())];
    report.insert((SOURCE.to_owned(), OTHER_FUNCTION.to_owned()), metrics);
    assert!(instances::parse(&input, report, Path::new(ROOT)).is_err());
    assert!(
        instances::parse(
            &data(vec![function(), function()]),
            crate::instance_fixture::report(),
            Path::new(ROOT)
        )
        .is_err()
    );
}

#[test]
fn each_metric_vector_must_agree_with_the_function_report() {
    for mutate in [
        |metrics: &mut FunctionMetrics| metrics.lines.hit = u64::default(),
        |metrics: &mut FunctionMetrics| metrics.regions.hit = u64::default(),
        |metrics: &mut FunctionMetrics| metrics.branches.hit = u64::default(),
    ] {
        let mut report = report();
        mutate(
            report
                .get_mut(&(SOURCE.to_owned(), FUNCTION.to_owned()))
                .unwrap(),
        );
        assert!(instances::parse(&data(vec![function()]), report, Path::new(ROOT)).is_err());
    }
}

#[test]
fn nonproduction_functions_do_not_create_instances() {
    let mut function = function();
    function.filenames[FILE_INDEX as usize] = TEST_SOURCE.to_owned();
    assert!(
        instances::parse(
            &data(vec![function]),
            FunctionReport::new(),
            Path::new(ROOT)
        )
        .unwrap()
        .is_empty()
    );
}

#[test]
fn every_summary_dimension_is_required_and_reconciled() {
    let parsed = instances::parse(&data(vec![function()]), report(), Path::new(ROOT)).unwrap();
    for kind in MetricKind::ALL {
        let mut missing = summaries();
        missing.get_mut(SOURCE).unwrap().remove(&kind);
        assert!(validation::validate(&missing, &parsed).is_err());
        let mut changed = summaries();
        changed
            .get_mut(SOURCE)
            .unwrap()
            .get_mut(&kind)
            .unwrap()
            .found += SINGLE_METRIC;
        assert!(validation::validate(&changed, &parsed).is_err());
    }
    assert!(validation::validate(&Default::default(), &parsed).is_err());
    assert!(validation::validate(&summaries(), &Default::default()).is_err());
}

#[test]
fn generic_function_instances_share_one_file_summary_group() {
    let mut other = function();
    other.name = OTHER_FUNCTION.to_owned();
    let mut report = report();
    let metrics = report[&(SOURCE.to_owned(), FUNCTION.to_owned())];
    report.insert((SOURCE.to_owned(), OTHER_FUNCTION.to_owned()), metrics);
    let parsed = instances::parse(&data(vec![function(), other]), report, Path::new(ROOT)).unwrap();
    validation::validate(&summaries(), &parsed).unwrap();
}

const LLVM_SATURATED_COUNT: u64 = i64::MAX as u64;
const LLVM_LARGEST_UNSATURATED_COUNT: u64 = LLVM_SATURATED_COUNT - 1;

#[test]
fn execution_counts_reject_saturated_and_wrapped_production_values() {
    use crate::coverage_json::{COUNT_INDEX, FALSE_COUNT_INDEX, Function};
    for value in [LLVM_SATURATED_COUNT, u64::MAX] {
        for mutate in [
            |function: &mut Function, count| function.count = count,
            |function: &mut Function, count| {
                function.regions[FILE_INDEX as usize][COUNT_INDEX] = count
            },
            |function: &mut Function, count| {
                function.branches[FILE_INDEX as usize][COUNT_INDEX] = count
            },
            |function: &mut Function, count| {
                function.branches[FILE_INDEX as usize][FALSE_COUNT_INDEX] = count
            },
        ] {
            let mut function = function();
            mutate(&mut function, value);
            let error =
                instances::parse(&data(vec![function]), report(), Path::new(ROOT)).unwrap_err();
            assert!(error.contains(SOURCE), "{error}");
            assert!(error.contains(FUNCTION), "{error}");
            assert!(error.contains(&value.to_string()), "{error}");
        }
    }
}

#[test]
fn execution_counts_keep_the_largest_unambiguous_value_without_clamping() {
    use crate::coverage_json::{COUNT_INDEX, FALSE_COUNT_INDEX};
    let mut function = function();
    function.count = LLVM_LARGEST_UNSATURATED_COUNT;
    function.regions[FILE_INDEX as usize][COUNT_INDEX] = LLVM_LARGEST_UNSATURATED_COUNT;
    function.branches[FILE_INDEX as usize][COUNT_INDEX] = LLVM_LARGEST_UNSATURATED_COUNT;
    function.branches[FILE_INDEX as usize][FALSE_COUNT_INDEX] = LLVM_LARGEST_UNSATURATED_COUNT;
    let parsed = instances::parse(&data(vec![function]), report(), Path::new(ROOT)).unwrap();
    let instance = &parsed[SOURCE][FILE_INDEX as usize];
    for vector in instance.vectors.values() {
        assert!(
            vector
                .iter()
                .all(|count| *count == LLVM_LARGEST_UNSATURATED_COUNT)
        );
    }
    validation::validate(&summaries(), &parsed).unwrap();
}

#[test]
fn execution_counts_do_not_promote_test_only_functions_into_production() {
    let mut function = function();
    function.filenames[FILE_INDEX as usize] = TEST_SOURCE.into();
    function.count = LLVM_SATURATED_COUNT;
    let parsed = instances::parse(
        &data(vec![function]),
        FunctionReport::new(),
        Path::new(ROOT),
    )
    .unwrap();
    assert!(parsed.is_empty());
}
