use crate::constants::*;
use std::path::{Component, Path, PathBuf};

pub struct Context {
    pub execution_root: PathBuf,
    pub llvm_cov: PathBuf,
    pub llvm_profdata: PathBuf,
    pub profile_data: PathBuf,
    pub raw_profiles: Vec<PathBuf>,
    pub report: PathBuf,
    pub instrumented_objects: Vec<PathBuf>,
}

impl Context {
    pub fn load() -> Result<Self, String> {
        Self::load_with_profiles(crate::raw_profiles::read)
    }

    pub(crate) fn load_with_profiles(
        read_profiles: impl FnOnce(&Path) -> Result<Vec<PathBuf>, String>,
    ) -> Result<Self, String> {
        let coverage_directory = variable_path(COVERAGE_DIRECTORY_ENV)?;
        let execution_root = variable_path(EXECUTION_ROOT_ENV)?;
        let runfiles = absolute_runfiles(&execution_root, variable_path(RUNFILES_DIRECTORY_ENV)?);
        let llvm_cov = metadata(&execution_root, &runfiles, LLVM_COV_ENV)?;
        let llvm_profdata = metadata(&execution_root, &runfiles, LLVM_PROFDATA_ENV)?;
        let instrumented_objects = instrumented_objects(&execution_root, &runfiles)?;
        let raw_profiles = read_profiles(&coverage_directory)?;
        Ok(Self {
            profile_data: coverage_directory.join(PROFILE_DATA_NAME),
            report: coverage_directory.join(COVERAGE_REPORT_NAME),
            execution_root,
            llvm_cov,
            llvm_profdata,
            raw_profiles,
            instrumented_objects,
        })
    }
}

fn variable(name: &str) -> Result<String, String> {
    std::env::var(name).map_err(|error| format!("missing or invalid {name}: {error}"))
}

fn variable_path(name: &str) -> Result<PathBuf, String> {
    variable(name).map(PathBuf::from)
}

fn absolute_runfiles(execution_root: &Path, runfiles: PathBuf) -> PathBuf {
    if runfiles.is_absolute() {
        runfiles
    } else {
        execution_root.join(runfiles)
    }
}

fn metadata(execution_root: &Path, runfiles: &Path, name: &str) -> Result<PathBuf, String> {
    let relative = variable_path(name)?;
    let execution_candidate = execution_root.join(&relative);
    if execution_candidate.is_file() {
        return Ok(execution_candidate);
    }
    let runfiles_candidate = runfiles.join(relative);
    runfiles_candidate
        .is_file()
        .then_some(runfiles_candidate)
        .ok_or_else(|| format!("{name} tool is unavailable"))
}

fn instrumented_objects(execution_root: &Path, runfiles: &Path) -> Result<Vec<PathBuf>, String> {
    let instrumented = optional_variable(INSTRUMENTED_TEST_BINARY_ENV)?;
    let bazel = instrumented
        .is_none()
        .then(|| variable(TEST_BINARY_ENV))
        .transpose()?;
    let primary = select_test_binary(instrumented.as_deref(), bazel.as_deref())?;
    let additional =
        select_additional_objects(optional_variable(ADDITIONAL_COVERAGE_OBJECTS_ENV)?.as_deref())?;
    let mut objects = Vec::with_capacity(additional.len() + PRIMARY_INSTRUMENTED_OBJECT_COUNT);
    objects.push(resolve_instrumented_object(
        execution_root,
        runfiles,
        &primary,
    )?);
    for object in additional {
        let resolved = resolve_instrumented_object(execution_root, runfiles, &object)?;
        if objects.contains(&resolved) {
            return Err("instrumented coverage object is duplicated".to_owned());
        }
        objects.push(resolved);
    }
    Ok(objects)
}

fn resolve_instrumented_object(
    execution_root: &Path,
    runfiles: &Path,
    relative: &Path,
) -> Result<PathBuf, String> {
    let runfile = runfiles.join(variable(TEST_WORKSPACE_ENV)?).join(&relative);
    if runfile.is_file() {
        return Ok(runfile);
    }
    let configuration = runfiles
        .strip_prefix(execution_root)
        .map_err(|_| "runfiles directory is outside the execution root".to_owned())?
        .components()
        .take(RUNFILES_CONFIGURATION_COMPONENTS)
        .collect::<PathBuf>();
    let fallback = execution_root.join(configuration).join(relative);
    fallback
        .is_file()
        .then_some(fallback)
        .ok_or_else(|| "instrumented coverage object is unavailable".to_owned())
}

fn optional_variable(name: &str) -> Result<Option<String>, String> {
    match std::env::var(name) {
        Ok(value) => Ok(Some(value)),
        Err(std::env::VarError::NotPresent) => Ok(None),
        Err(error) => Err(format!("missing or invalid {name}: {error}")),
    }
}

pub(crate) fn select_test_binary(
    instrumented: Option<&str>,
    bazel: Option<&str>,
) -> Result<PathBuf, String> {
    let (name, value) = instrumented
        .map(|value| (INSTRUMENTED_TEST_BINARY_ENV, value))
        .or_else(|| bazel.map(|value| (TEST_BINARY_ENV, value)))
        .ok_or_else(|| "instrumented test binary environment is unavailable".to_owned())?;
    let path = PathBuf::from(value);
    let segment_count = value.split(std::path::MAIN_SEPARATOR).count();
    let valid = !value.is_empty()
        && !path.is_absolute()
        && path.components().count() == segment_count
        && path
            .components()
            .all(|component| matches!(component, Component::Normal(_)));
    valid
        .then_some(path)
        .ok_or_else(|| format!("{name} must be a workspace-relative normalized path"))
}

pub(crate) fn select_additional_objects(value: Option<&str>) -> Result<Vec<PathBuf>, String> {
    let Some(value) = value else {
        return Ok(Vec::new());
    };
    let values: Vec<String> = serde_json::from_str(value)
        .map_err(|error| format!("{ADDITIONAL_COVERAGE_OBJECTS_ENV} is invalid: {error}"))?;
    values
        .into_iter()
        .map(|value| normalized_workspace_path(ADDITIONAL_COVERAGE_OBJECTS_ENV, &value))
        .collect()
}

fn normalized_workspace_path(name: &str, value: &str) -> Result<PathBuf, String> {
    let path = PathBuf::from(value);
    let segment_count = value.split(std::path::MAIN_SEPARATOR).count();
    let valid = !value.is_empty()
        && !path.is_absolute()
        && path.components().count() == segment_count
        && path
            .components()
            .all(|component| matches!(component, Component::Normal(_)));
    valid
        .then_some(path)
        .ok_or_else(|| format!("{name} must contain workspace-relative normalized paths"))
}
