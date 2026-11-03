use crate::{
    BUILD_SYSTEM_SOURCE_ROOT, CURRENT_DIRECTORY_COMPONENT, CURRENT_DIRECTORY_PREFIX,
    FIXTURE_DIRECTORY, GENERATED_DIRECTORY, PARENT_DIRECTORY_COMPONENT, PLUGIN_SOURCE_ROOT,
    RUST_COMPONENT_SOURCE_ROOT, RUST_SOURCE_EXTENSION, RUST_SOURCE_ROOT, SOURCE_DIRECTORY,
    TEST_DIRECTORY, TEST_SOURCE, TEST_SOURCE_PREFIX, TEST_SUPPORT_DIRECTORY,
    TEST_SUPPORT_PACKAGE_SUFFIX, TESTS_DIRECTORY, TESTS_SOURCE, TESTS_SOURCE_PREFIX,
    TESTS_SOURCE_SUFFIX, TESTS_SUFFIX,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProductionScope {
    Core,
    Tooling,
}

pub fn production_key(source: &str) -> Result<Option<String>, String> {
    Ok(classify(source)?.map(|(_, source)| source.to_owned()))
}

pub fn production_scope(source: &str) -> Result<Option<ProductionScope>, String> {
    Ok(classify(source)?.map(|(scope, _)| scope))
}

fn classify(source: &str) -> Result<Option<(ProductionScope, &str)>, String> {
    let source = source
        .strip_prefix(CURRENT_DIRECTORY_PREFIX)
        .unwrap_or(source);
    let Some((scope, relative)) = [
        (RUST_SOURCE_ROOT, ProductionScope::Core),
        (RUST_COMPONENT_SOURCE_ROOT, ProductionScope::Core),
        (PLUGIN_SOURCE_ROOT, ProductionScope::Core),
        (BUILD_SYSTEM_SOURCE_ROOT, ProductionScope::Tooling),
    ]
    .into_iter()
    .find_map(|(root, scope)| source.strip_prefix(root).map(|relative| (scope, relative))) else {
        return Ok(None);
    };
    let components = relative.split('/').collect::<Vec<_>>();
    if components.is_empty()
        || components.iter().any(|component| {
            component.is_empty()
                || *component == CURRENT_DIRECTORY_COMPONENT
                || *component == PARENT_DIRECTORY_COMPONENT
        })
    {
        return Err(format!("invalid Rust source path {source:?}"));
    }
    let (file, directories) = components
        .split_last()
        .expect("validated source components");
    if directories
        .iter()
        .any(|component| test_directory(component))
        || (scope == ProductionScope::Core && test_source(file))
    {
        return Ok(None);
    }
    if (scope == ProductionScope::Tooling || source.starts_with(PLUGIN_SOURCE_ROOT))
        && (!file.ends_with(RUST_SOURCE_EXTENSION)
            || !directories.contains(&SOURCE_DIRECTORY)
            || directories
                .iter()
                .any(|component| tooling_support(component)))
    {
        return Ok(None);
    }
    Ok(Some((scope, source)))
}

fn tooling_support(component: &str) -> bool {
    component == FIXTURE_DIRECTORY
        || component == GENERATED_DIRECTORY
        || component.ends_with(TEST_SUPPORT_PACKAGE_SUFFIX)
}

fn test_directory(component: &str) -> bool {
    component == TEST_DIRECTORY
        || component == TESTS_DIRECTORY
        || component == TEST_SUPPORT_DIRECTORY
        || component.ends_with(TESTS_SUFFIX)
}

fn test_source(file: &str) -> bool {
    file == TEST_SOURCE
        || file == TESTS_SOURCE
        || file.ends_with(TESTS_SOURCE_SUFFIX)
        || (file.ends_with(RUST_SOURCE_EXTENSION)
            && (file.starts_with(TEST_SOURCE_PREFIX) || file.starts_with(TESTS_SOURCE_PREFIX)))
}
