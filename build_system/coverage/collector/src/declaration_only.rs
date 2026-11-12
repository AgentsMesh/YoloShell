use crate::constants::{
    COVERAGE_MANIFEST_ENV, DECLARATION_ONLY_DETAIL, RUNFILES_DIRECTORY_ENV, TEST_ONLY_DETAIL,
    TEST_WORKSPACE_ENV,
};
use crate::coverage_json::Data;
use crate::environment::Context;
use crate::{declaration_sources, lcov, normalize};
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use yolo_coverage_protocol::production_key;
use yolo_coverage_source_contract::{
    SourceContract, is_external_rust_source, is_test_source, verify_manifest,
};

pub(crate) fn export(
    context: &Context,
    data: &Data,
    report: &str,
) -> Result<Option<String>, String> {
    if has_production_files(data, &context.execution_root)? {
        return Ok(None);
    }
    let manifest = std::env::var_os(COVERAGE_MANIFEST_ENV)
        .map(PathBuf::from)
        .ok_or_else(|| "declaration-only coverage requires Bazel's source manifest".to_owned())?;
    let bytes = declaration_sources::read_bounded(&context.execution_root.join(manifest))?;
    let manifest = String::from_utf8(bytes)
        .map_err(|error| format!("invalid coverage source manifest: {error}"))?;
    let source_root = declaration_sources::runfile_root(
        &context.execution_root,
        &required_path(RUNFILES_DIRECTORY_ENV)?,
        &required_path(TEST_WORKSPACE_ENV)?,
    )?;
    let contract = validate(data, &context.execution_root, &manifest, |source| {
        declaration_sources::read_bounded(&source_root.join(source))
    })?;
    let output = lcov::augment(report, &Default::default(), &BTreeSet::new())?;
    let detail = match contract {
        SourceContract::Declarations => DECLARATION_ONLY_DETAIL,
        SourceContract::TestsOnly => TEST_ONLY_DETAIL,
    };
    eprintln!("{detail}");
    Ok(Some(output))
}

fn required_path(name: &str) -> Result<PathBuf, String> {
    std::env::var_os(name)
        .map(PathBuf::from)
        .ok_or_else(|| format!("declaration-only coverage requires {name}"))
}

pub(crate) fn has_production_files(data: &Data, root: &Path) -> Result<bool, String> {
    for file in &data.files {
        if production_key(&mapped_source(&file.filename, root))?.is_some() {
            return Ok(true);
        }
    }
    Ok(false)
}

pub(crate) fn validate(
    data: &Data,
    root: &Path,
    manifest: &str,
    read_source: impl FnMut(&str) -> Result<Vec<u8>, String>,
) -> Result<SourceContract, String> {
    let contract = verify_manifest(manifest, read_source)?;
    if has_production_files(data, root)? {
        return Err("declaration-only admission cannot replace production mappings".to_owned());
    }
    let declared = manifest
        .lines()
        .map(|line| line.trim_end_matches('\r'))
        .collect::<BTreeSet<_>>();
    if contract == SourceContract::TestsOnly {
        for filename in data.files.iter().map(|file| &file.filename).chain(
            data.functions
                .iter()
                .flat_map(|function| &function.filenames)
                .filter(|filename| !is_external_rust_source(filename, root)),
        ) {
            let source = mapped_source(filename, root);
            if !declared.contains(source.as_str()) || !is_test_source(&source, true)? {
                return Err(format!(
                    "test-only coverage contains an undeclared source mapping: {source:?} from {filename:?}"
                ));
            }
        }
    }
    let mut witnessed_test = false;
    for function in &data.functions {
        for (file_id, filename) in function.filenames.iter().enumerate() {
            let source = mapped_source(filename, root);
            if production_key(&source)?.is_some() {
                return Err("production function mapping has no file summary".to_owned());
            }
            if function.count > 0
                && !function.name.is_empty()
                && !function.regions.is_empty()
                && declared.contains(source.as_str())
                && is_test_source(&source, false)?
                && executed_test_region(function, file_id)
                && executed_test_file(data, root, &source)
            {
                witnessed_test = true;
            }
        }
    }
    if !witnessed_test {
        return Err("declaration-only coverage lacks executed test instrumentation".to_owned());
    }
    Ok(contract)
}

fn executed_test_region(function: &crate::coverage_json::Function, file_id: usize) -> bool {
    use crate::coverage_json::{CODE_REGION_KIND, COUNT_INDEX, FILE_ID_INDEX, KIND_INDEX};
    function.regions.iter().any(|region| {
        usize::try_from(region[FILE_ID_INDEX]) == Ok(file_id)
            && region[KIND_INDEX] == CODE_REGION_KIND
            && region[COUNT_INDEX] > 0
    })
}

fn executed_test_file(data: &Data, root: &Path, source: &str) -> bool {
    data.files.iter().any(|file| {
        mapped_source(&file.filename, root) == source
            && file.summary.functions.covered > 0
            && [
                &file.summary.lines,
                &file.summary.functions,
                &file.summary.regions,
                &file.summary.branches,
            ]
            .into_iter()
            .all(|metric| metric.covered <= metric.count)
    })
}

fn mapped_source(source: &str, root: &Path) -> String {
    yolo_coverage_source_contract::mapped_source(&normalize::source(source, root))
}
