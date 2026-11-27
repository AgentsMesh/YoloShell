mod commands;
mod completion;
mod constants;
mod coverage_data;
mod coverage_json;
mod declaration_only;
mod declaration_sources;
mod environment;
mod function_report;
mod function_report_fields;
mod instance_geometry;
mod instances;
mod inventory;
mod lcov;
mod lcov_synthetic;
mod lcov_write;
mod model;
mod normalize;
mod raw_profiles;
mod segment_lines;
mod segments;
mod signature;
mod validation;

#[cfg(all(test, not(feature = "yolo-integration-tests")))]
#[path = "../unit_tests/commands_tests.rs"]
mod commands_tests;
#[cfg(all(test, not(feature = "yolo-integration-tests")))]
#[path = "../unit_tests/completion_tests.rs"]
mod completion_tests;
#[cfg(all(test, not(feature = "yolo-integration-tests")))]
#[path = "../unit_tests/coverage_summary_tests.rs"]
mod coverage_summary_tests;
#[cfg(all(test, not(feature = "yolo-integration-tests")))]
#[path = "../unit_tests/declaration_external_tests.rs"]
mod declaration_external_tests;
#[cfg(test)]
#[path = "../test_support/declaration_fixture.rs"]
mod declaration_fixture;
#[cfg(all(test, not(feature = "yolo-integration-tests")))]
#[path = "../unit_tests/declaration_only_tests.rs"]
mod declaration_only_tests;
#[cfg(all(test, not(feature = "yolo-integration-tests")))]
#[path = "../unit_tests/environment_tests.rs"]
mod environment_tests;
#[cfg(all(test, not(feature = "yolo-integration-tests")))]
#[path = "../unit_tests/function_report_adversarial_tests.rs"]
mod function_report_adversarial_tests;
#[cfg(all(test, not(feature = "yolo-integration-tests")))]
#[path = "../unit_tests/function_report_tests.rs"]
mod function_report_tests;
#[cfg(test)]
#[path = "../test_support/instance_fixture.rs"]
mod instance_fixture;
#[cfg(all(test, not(feature = "yolo-integration-tests")))]
#[path = "../unit_tests/instance_geometry_tests.rs"]
mod instance_geometry_tests;
#[cfg(all(test, not(feature = "yolo-integration-tests")))]
#[path = "../unit_tests/instance_pipeline_tests.rs"]
mod instance_pipeline_tests;
#[cfg(all(test, not(feature = "yolo-integration-tests")))]
#[path = "../unit_tests/lcov_tests.rs"]
mod lcov_tests;
#[cfg(all(test, feature = "yolo-integration-tests"))]
#[path = "../integration_tests/merger_protocol_tests.rs"]
mod merger_protocol_tests;
#[cfg(all(test, not(feature = "yolo-integration-tests")))]
#[path = "../unit_tests/normalize_tests.rs"]
mod normalize_tests;
#[cfg(all(test, not(feature = "yolo-integration-tests")))]
#[path = "../unit_tests/scope_tests.rs"]
mod scope_tests;
#[cfg(all(test, not(feature = "yolo-integration-tests")))]
#[path = "../unit_tests/segments_tests.rs"]
mod segments_tests;
#[cfg(all(test, not(feature = "yolo-integration-tests")))]
#[path = "../unit_tests/signature_tests.rs"]
mod signature_tests;

use commands::ExportFormat;
use constants::{FAILURE_EXIT_CODE, VERBOSE_ENV};
use environment::Context;

pub fn run() {
    run_with_context(Context::load);
}

fn run_with_context(load: impl FnOnce() -> Result<Context, String>) {
    let outcome = completion::Completion::capture()
        .map_err(|error| format!("cannot establish coverage completion marker: {error}"))
        .and_then(|completion| {
            collect(load)?;
            completion
                .finish()
                .map_err(|error| format!("cannot confirm coverage completion: {error}"))
        });
    if let Err(error) = outcome {
        eprintln!("Rust coverage collection failed: {error}");
        std::process::exit(FAILURE_EXIT_CODE);
    }
}

fn collect(load: impl FnOnce() -> Result<Context, String>) -> Result<(), String> {
    let context = load()?;
    debug(&format!(
        "collecting {} raw profiles for {} instrumented objects",
        context.raw_profiles.len(),
        context.instrumented_objects.len()
    ));
    commands::merge_profiles(&context)?;
    let result = export_reports(&context);
    let cleanup = std::fs::remove_file(&context.profile_data).map_err(|error| {
        format!(
            "cannot remove intermediate {}: {error}",
            context.profile_data.display()
        )
    });
    let report = result?;
    cleanup?;
    std::fs::write(&context.report, report)
        .map_err(|error| format!("cannot write {}: {error}", context.report.display()))
}

fn export_reports(context: &Context) -> Result<String, String> {
    let lcov = commands::export(context, ExportFormat::Lcov)?;
    let json = commands::export(context, ExportFormat::Json)?;
    let normalized_lcov = normalize::report(&lcov, &context.execution_root);
    let data = coverage_json::parse(&json)?;
    coverage_data::reject_staged_sources(context, &data, &normalized_lcov)?;
    if let Some(report) = declaration_only::export(context, &data, &normalized_lcov)? {
        return Ok(report);
    }
    let summaries = coverage_data::summaries(&data, &context.execution_root)?;
    let sources = coverage_data::sources(&summaries);
    let report = commands::function_report(context, &sources)?;
    let report = function_report::parse(&report, &context.execution_root)?;
    let instances = instances::parse(&data, report, &context.execution_root)?;
    validation::validate(&summaries, &instances)?;
    let expected_sources = sources.into_iter().collect();
    lcov::augment(&normalized_lcov, &instances, &expected_sources)
}

fn debug(message: &str) {
    if std::env::var_os(VERBOSE_ENV).is_some() {
        eprintln!("{message}");
    }
}

#[cfg(all(test, feature = "yolo-integration-tests"))]
#[path = "../test_support/declaration_pipeline_fixture.rs"]
mod declaration_pipeline_fixture;
#[cfg(all(test, feature = "yolo-integration-tests"))]
#[path = "../integration_tests/declaration_pipeline_tests.rs"]
mod declaration_pipeline_tests;
#[cfg(all(test, feature = "yolo-integration-tests"))]
#[path = "../test_support/entrypoint_fixture.rs"]
mod entrypoint_fixture;
#[cfg(all(test, feature = "yolo-integration-tests"))]
#[path = "../integration_tests/entrypoint_tests.rs"]
mod entrypoint_tests;
#[cfg(all(test, feature = "yolo-integration-tests"))]
#[path = "../integration_tests/profile_inventory.rs"]
mod profile_inventory;
#[cfg(all(test, feature = "yolo-integration-tests"))]
#[path = "../integration_tests/repository_test_only.rs"]
mod repository_test_only_integration_tests;
#[cfg(all(test, feature = "yolo-integration-tests"))]
#[path = "../integration_tests/staged_sources.rs"]
mod staged_sources_integration_tests;
#[cfg(test)]
#[path = "../test_support/constants.rs"]
mod test_constants;
