use crate::constants::{
    BUILD_SYSTEM_PREFIX, GENERATED_TEXT_FILE_NAMES, HANDWRITTEN_EXTENSIONS, HANDWRITTEN_FILE_NAMES,
    INTEGRATION_TEST_DIRECTORY_NAME, RETIRED_FRONTEND_DIRECTORY_PARTS, RETIRED_FRONTEND_FILE_PARTS,
    RETIRED_SCRIPT_EXTENSIONS, RUST_TEST_FILE_SUFFIXES, TEST_SUPPORT_DIRECTORY_NAME,
    TEST_SUPPORT_FILE_STEMS, UNIT_TEST_DIRECTORY_NAME,
};
use std::path::Path;
use yolo_architecture_foundation::{SourceFile, model::Violation, text::joined};

const RUST_SOURCE_TEST_MESSAGE: &str =
    "Rust test implementations must live in unit_tests or integration_tests";
const MISPLACED_TEST_SUPPORT_MESSAGE: &str =
    "Rust test fixtures and support must live in test_support";
const TEST_SUPPORT_TEST_MESSAGE: &str =
    "Rust test_support sources must not contain executable tests";

pub fn inspect(file: &SourceFile, violations: &mut Vec<Violation>) {
    if rust_implementation_source(&file.relative)
        && (is_rust_test_source(&file.relative) || contains_test_declaration(&file.text))
    {
        violations.push(Violation::file(&file.relative, RUST_SOURCE_TEST_MESSAGE));
    }
    if misplaced_test_support(&file.relative) {
        violations.push(Violation::file(
            &file.relative,
            MISPLACED_TEST_SUPPORT_MESSAGE,
        ));
    }
    if directory_component(&file.relative, TEST_SUPPORT_DIRECTORY_NAME)
        && contains_test_declaration(&file.text)
    {
        violations.push(Violation::file(&file.relative, TEST_SUPPORT_TEST_MESSAGE));
    }
}

fn misplaced_test_support(relative: &str) -> bool {
    let path = Path::new(relative);
    if !relative.starts_with(BUILD_SYSTEM_PREFIX)
        || path.extension().and_then(|value| value.to_str()) != Some("rs")
        || !(directory_component(relative, UNIT_TEST_DIRECTORY_NAME)
            || directory_component(relative, INTEGRATION_TEST_DIRECTORY_NAME))
    {
        return false;
    }
    let stem = file_stem(path);
    let executable_test = RUST_TEST_FILE_SUFFIXES
        .iter()
        .any(|suffix| stem.ends_with(suffix));
    !executable_test
        && TEST_SUPPORT_FILE_STEMS
            .iter()
            .any(|marker| stem == *marker || stem.contains(marker))
}

fn directory_component(relative: &str, expected: &str) -> bool {
    Path::new(relative)
        .components()
        .any(|component| component.as_os_str() == expected)
}

pub fn is_handwritten(path: &Path) -> bool {
    let name = file_name(path);
    if GENERATED_TEXT_FILE_NAMES.contains(&name) {
        return false;
    }
    HANDWRITTEN_FILE_NAMES.contains(&name)
        || path
            .extension()
            .and_then(|value| value.to_str())
            .is_some_and(|extension| HANDWRITTEN_EXTENSIONS.contains(&extension))
}

pub use yolo_architecture_foundation::source_path::is_rust_test_source;

pub fn is_rust_source(relative: &str) -> bool {
    relative.ends_with(".rs")
        && (relative.starts_with("core/src/") || relative.starts_with("core/crates/"))
}

fn rust_implementation_source(relative: &str) -> bool {
    if !relative.ends_with(".rs") {
        return false;
    }
    relative.starts_with("core/src/")
        || (relative.starts_with("core/crates/")
            && relative.contains("/src/")
            && !relative.contains("/unit_tests/")
            && !relative.contains("/integration_tests/"))
        || (relative.starts_with(BUILD_SYSTEM_PREFIX)
            && relative.contains("/src/")
            && !relative.contains("/unit_tests/")
            && !relative.contains("/integration_tests/")
            && !relative.contains("/test_support/"))
}

fn contains_test_declaration(text: &str) -> bool {
    crate::rust_test_declarations::contains(text)
}

pub fn is_rust_product_source(relative: &str) -> bool {
    is_rust_source(relative) && !is_rust_test_source(relative)
}

pub fn is_retired_frontend_file(path: &Path) -> bool {
    let retired_extension = path
        .extension()
        .and_then(|value| value.to_str())
        .is_some_and(|extension| RETIRED_SCRIPT_EXTENSIONS.contains(&extension));
    let name = file_name(path);
    retired_extension
        || RETIRED_FRONTEND_FILE_PARTS
            .iter()
            .any(|parts| name == joined(parts))
}

pub fn is_retired_frontend_directory(path: &Path) -> bool {
    let name = file_name(path);
    RETIRED_FRONTEND_DIRECTORY_PARTS
        .iter()
        .any(|parts| name == joined(parts))
}

fn file_name(path: &Path) -> &str {
    path.file_name()
        .and_then(|value| value.to_str())
        .unwrap_or_default()
}

fn file_stem(path: &Path) -> &str {
    path.file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or_default()
}

#[cfg(test)]
#[path = "../unit_tests/source_boundary_tests.rs"]
mod boundary_tests;
#[cfg(test)]
#[path = "../unit_tests/jsonl_policy_tests.rs"]
mod jsonl_tests;
#[cfg(test)]
#[path = "../unit_tests/source_policy_tests.rs"]
mod tests;
