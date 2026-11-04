#[path = "../test_support/commands.rs"]
mod commands;
#[path = "../test_support/constants.rs"]
mod constants;
#[path = "../test_support/layout_commands.rs"]
mod layout_commands;
#[path = "../test_support/macho.rs"]
mod macho;
#[path = "../test_support/resources.rs"]
mod resources;
use constants::*;
use layout_commands::{collect, function};

#[test]
fn optimized_counters_are_isolated_from_unknown_data_sections() {
    inspect_probe(O3_PROBE_ENV, false);
}

#[test]
fn mcdc_probe_contains_a_real_nonempty_isolated_bitmap() {
    inspect_probe(MCDC_PROBE_ENV, true);
}

#[tokio::test]
async fn optimized_unknown_data_survives_continuous_normal_and_sigkill_exits() {
    let ordinary = collect(O3_PROBE_ENV, ORDINARY_PROFILE_PATTERN, NORMAL, true)
        .await
        .unwrap();
    let expected = function(&ordinary, O3_OBSERVED, O3_CALLS);
    for mode in [NORMAL, BLOCKED] {
        let continuous = collect(O3_PROBE_ENV, PROFILE_PATTERN, mode, true)
            .await
            .unwrap();
        assert_eq!(function(&continuous, O3_OBSERVED, O3_CALLS), expected);
    }
}

#[tokio::test]
async fn optimized_bazel_pipeline_retains_coverage_only_when_enabled() {
    let enabled = resources::settings()[COVERAGE_FIELD].as_bool().unwrap();
    if enabled {
        inspect_probe(O3_PIPELINE_ENV, false);
    }
    let report = collect(O3_PIPELINE_ENV, PROFILE_PATTERN, NORMAL, enabled).await;
    match report {
        Some(report) => {
            function(&report, O3_OBSERVED, O3_CALLS);
        }
        None => assert!(!enabled),
    }
}

#[tokio::test]
async fn nonempty_mcdc_bitmap_preserves_all_conditions_after_sigkill() {
    let ordinary = collect(MCDC_PROBE_ENV, ORDINARY_PROFILE_PATTERN, NORMAL, true)
        .await
        .unwrap();
    let expected = function(&ordinary, MCDC_OBSERVED, MCDC_CALLS);
    let records = expected[MCDC_RECORDS_FIELD].as_array().unwrap();
    assert!(!records.is_empty(), "{expected}");
    assert!(
        records.iter().any(|record| {
            let conditions = record[MCDC_CONDITIONS_INDEX].as_array().unwrap();
            conditions.len() == MCDC_CONDITIONS
                && conditions.iter().all(|value| value.as_bool() == Some(true))
        }),
        "{expected}"
    );
    for mode in [NORMAL, BLOCKED] {
        let continuous = collect(MCDC_PROBE_ENV, PROFILE_PATTERN, mode, true)
            .await
            .unwrap();
        assert_eq!(function(&continuous, MCDC_OBSERVED, MCDC_CALLS), expected);
    }
}

fn inspect_probe(variable: &str, nonempty_bitmap: bool) {
    let binary = resources::binary(variable);
    let image = macho::inspect(&std::fs::read(&binary).unwrap()).unwrap();
    macho::verify_profile_layout(&image)
        .unwrap_or_else(|error| panic!("{}: {error}", binary.display()));
    assert!(image.segments.iter().any(|segment| {
        segment.name == DATA_SEGMENT
            && segment
                .sections
                .iter()
                .any(|section| section.name == UNKNOWN_SECTION && section.size > NOT_EXECUTED)
    }));
    if nonempty_bitmap {
        assert!(image.segments.iter().any(|segment| {
            segment.name == BITMAP_SEGMENT
                && segment
                    .sections
                    .iter()
                    .any(|section| section.name == BITMAP_SECTION && section.size > NOT_EXECUTED)
        }));
    }
}
