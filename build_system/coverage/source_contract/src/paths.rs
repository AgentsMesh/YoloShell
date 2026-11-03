use crate::constants::{
    COVERAGE_TOOL_ARTIFACT_SUFFIXES, DECLARATION_BAZEL_BINARY, DECLARATION_BAZEL_OUTPUT,
    DECLARATION_SOURCE_EXTENSION, DECLARATION_TEST_DIRECTORIES, EXTERNAL_ARTIFACT_DIRECTORY,
    INVALID_REPOSITORY_TEST_PATH, REPOSITORY_TEST_ROOT, TEST_SUPPORT_DIRECTORY,
};
use std::path::{Component, Path};
use yolo_coverage_protocol::{
    BUILD_SYSTEM_SOURCE_ROOT, PLUGIN_SOURCE_ROOT, RUST_COMPONENT_SOURCE_ROOT, RUST_SOURCE_ROOT,
    production_key,
};

pub fn is_test_source(source: &str, allow_support: bool) -> Result<bool, String> {
    if production_key(source)?.is_some() {
        return Ok(false);
    }
    let repository_test = source.strip_prefix(REPOSITORY_TEST_ROOT);
    if repository_test.is_some() && !crate::staged_paths::normalized_path(source) {
        return Err(INVALID_REPOSITORY_TEST_PATH.into());
    }
    let Some(relative) = repository_test.or_else(|| {
        [
            RUST_SOURCE_ROOT,
            RUST_COMPONENT_SOURCE_ROOT,
            PLUGIN_SOURCE_ROOT,
            BUILD_SYSTEM_SOURCE_ROOT,
        ]
        .into_iter()
        .find_map(|root| source.strip_prefix(root))
    }) else {
        return Ok(false);
    };
    let path = Path::new(relative);
    if path
        .extension()
        .is_none_or(|value| value != DECLARATION_SOURCE_EXTENSION)
    {
        return Ok(false);
    }
    // Repository integration fixtures are test inputs, but only explicit test
    // directories can witness an executed test when allow_support is false.
    if repository_test.is_some() && allow_support {
        return Ok(true);
    }
    Ok(path.parent().is_some_and(|parent| {
        parent.components().any(|component| {
            component.as_os_str().to_str().is_some_and(|part| {
                DECLARATION_TEST_DIRECTORIES.contains(&part)
                    || (allow_support && part == TEST_SUPPORT_DIRECTORY)
            })
        })
    }))
}

pub fn mapped_source(source: &str) -> String {
    let mut components = Path::new(source).components();
    if components.next()
        == Some(Component::Normal(std::ffi::OsStr::new(
            DECLARATION_BAZEL_OUTPUT,
        )))
        && matches!(components.next(), Some(Component::Normal(_)))
        && components.next()
            == Some(Component::Normal(std::ffi::OsStr::new(
                DECLARATION_BAZEL_BINARY,
            )))
        && components
            .clone()
            .all(|part| matches!(part, Component::Normal(_)))
    {
        return components.as_path().to_string_lossy().into_owned();
    }
    source.to_owned()
}

pub(crate) fn ancillary(source: &str) -> bool {
    if !crate::staged_paths::normalized_path(source) {
        return false;
    }
    let path = Path::new(source);
    let mut components = path.components();
    let Some(Component::Normal(root)) = components.next() else {
        return false;
    };
    if !components
        .clone()
        .all(|part| matches!(part, Component::Normal(_)))
    {
        return false;
    }
    if root == EXTERNAL_ARTIFACT_DIRECTORY {
        return crate::coverage_artifacts::pinned_distribution(source)
            || COVERAGE_TOOL_ARTIFACT_SUFFIXES
                .iter()
                .any(|suffix| source.ends_with(suffix));
    }
    root == DECLARATION_BAZEL_OUTPUT
        && components.next().is_some()
        && components
            .next()
            .is_some_and(|part| part.as_os_str() == DECLARATION_BAZEL_BINARY)
        && components.next().is_some()
        && path.extension().is_none()
}
