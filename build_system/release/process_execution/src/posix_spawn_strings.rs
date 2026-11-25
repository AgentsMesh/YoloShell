use crate::constants::{
    DETERMINISTIC_LOCALE, DISABLED_ENVIRONMENT_VALUE, ENABLED_ENVIRONMENT_VALUE, FIXED_SYSTEM_PATH,
    GIT_ATTR_NOSYSTEM_ENVIRONMENT, GIT_CONFIG_GLOBAL_ENVIRONMENT, GIT_CONFIG_NOSYSTEM_ENVIRONMENT,
    GIT_CONFIG_SYSTEM_ENVIRONMENT, GIT_NO_REPLACE_OBJECTS_ENVIRONMENT,
    GIT_OPTIONAL_LOCKS_ENVIRONMENT, HOME_ENVIRONMENT, LANG_ENVIRONMENT, LC_ALL_ENVIRONMENT,
    NULL_DEVICE, PATH_ENVIRONMENT, TEMP_ENVIRONMENT,
};
use crate::process::ProcessCommand;
use anyhow::Result;
use std::ffi::CString;
use std::ptr;
use yolo_process::{PROCESS_DISCOVERY_CAPABILITY_ENV, ProcessDiscoveryCapability};

pub(crate) struct CStringVector {
    _values: Vec<CString>,
    pub pointers: Vec<*mut libc::c_char>,
}

pub(crate) fn argument_vector(specification: &ProcessCommand) -> Result<CStringVector> {
    strings(
        std::iter::once(specification.executable.to_string_lossy().into_owned())
            .chain(specification.arguments.clone()),
    )
}

pub(crate) fn environment_vector(
    specification: &ProcessCommand,
    capability: &ProcessDiscoveryCapability,
) -> Result<CStringVector> {
    strings([
        format!("{PATH_ENVIRONMENT}={FIXED_SYSTEM_PATH}"),
        format!(
            "{HOME_ENVIRONMENT}={}",
            specification.home.display_path().display()
        ),
        format!(
            "{TEMP_ENVIRONMENT}={}",
            specification.temporary_directory.display_path().display()
        ),
        format!("{LANG_ENVIRONMENT}={DETERMINISTIC_LOCALE}"),
        format!("{LC_ALL_ENVIRONMENT}={DETERMINISTIC_LOCALE}"),
        format!("{GIT_CONFIG_NOSYSTEM_ENVIRONMENT}={ENABLED_ENVIRONMENT_VALUE}"),
        format!("{GIT_ATTR_NOSYSTEM_ENVIRONMENT}={ENABLED_ENVIRONMENT_VALUE}"),
        format!("{GIT_CONFIG_GLOBAL_ENVIRONMENT}={NULL_DEVICE}"),
        format!("{GIT_CONFIG_SYSTEM_ENVIRONMENT}={NULL_DEVICE}"),
        format!("{GIT_NO_REPLACE_OBJECTS_ENVIRONMENT}={ENABLED_ENVIRONMENT_VALUE}"),
        format!("{GIT_OPTIONAL_LOCKS_ENVIRONMENT}={DISABLED_ENVIRONMENT_VALUE}"),
        format!("{PROCESS_DISCOVERY_CAPABILITY_ENV}={}", capability.value()),
    ])
}

fn strings(values: impl IntoIterator<Item = String>) -> Result<CStringVector> {
    let values = values
        .into_iter()
        .map(CString::new)
        .collect::<std::result::Result<Vec<_>, _>>()?;
    let mut pointers = values
        .iter()
        .map(|value| value.as_ptr().cast_mut())
        .collect::<Vec<_>>();
    pointers.push(ptr::null_mut());
    Ok(CStringVector {
        _values: values,
        pointers,
    })
}

#[cfg(all(test, feature = "yolo-unit-tests"))]
#[path = "../unit_tests/strings.rs"]
mod unit_tests;
