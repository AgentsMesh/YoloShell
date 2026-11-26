use super::CStringVector;
use anyhow::{Result, ensure};
use std::collections::BTreeMap;
use std::ffi::{CString, OsString};
use std::os::unix::ffi::OsStrExt;

const ENVIRONMENT_SEPARATOR: u8 = b'=';
const NUL_BYTE: u8 = 0;
const INVALID_KEY: &str = "worker environment key is invalid";

pub(super) fn environment_vector(
    overrides: &[(OsString, Option<OsString>)],
) -> Result<CStringVector> {
    environment_vector_from(std::env::vars_os(), overrides)
}

fn environment_vector_from(
    inherited: impl IntoIterator<Item = (OsString, OsString)>,
    overrides: &[(OsString, Option<OsString>)],
) -> Result<CStringVector> {
    let mut environment: BTreeMap<_, _> = inherited.into_iter().collect();
    for (key, value) in overrides {
        ensure!(
            !key.is_empty()
                && !key.as_bytes().contains(&ENVIRONMENT_SEPARATOR)
                && !key.as_bytes().contains(&NUL_BYTE),
            INVALID_KEY
        );
        if let Some(value) = value {
            environment.insert(key.clone(), value.clone());
        } else {
            environment.remove(key);
        }
    }
    let values = environment
        .into_iter()
        .map(|(key, value)| {
            CString::new(
                key.as_bytes()
                    .iter()
                    .copied()
                    .chain(std::iter::once(ENVIRONMENT_SEPARATOR))
                    .chain(value.as_bytes().iter().copied())
                    .collect::<Vec<_>>(),
            )
        })
        .collect::<std::result::Result<Vec<_>, _>>()?;
    let mut pointers = values
        .iter()
        .map(|value| value.as_ptr().cast_mut())
        .collect::<Vec<_>>();
    pointers.push(std::ptr::null_mut());
    Ok(CStringVector {
        _values: values,
        pointers,
    })
}

#[cfg(all(test, feature = "yolo-unit-tests"))]
#[path = "../../unit_tests/worker_spawn_environment_tests.rs"]
mod tests;
