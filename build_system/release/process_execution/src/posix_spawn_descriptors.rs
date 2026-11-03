use crate::constants::FCNTL_ERROR;
use crate::process::ProcessCommand;
use anyhow::{Context, Result};
use std::fs::File;
use std::os::fd::{AsRawFd, FromRawFd, RawFd};

pub(crate) const DESCRIPTOR_RANGE_ERROR: &str = "child descriptor range is exhausted";
const DESCRIPTOR_DUPLICATION_CONTEXT: &str = "isolate spawn descriptor source";
const NEXT_DESCRIPTOR_OFFSET: libc::c_int = 1;

pub(crate) struct IsolatedDescriptorBinding {
    pub source: File,
    pub child_descriptor: libc::c_int,
}

pub(crate) struct IsolatedSpawnDescriptors {
    pub standard_output: File,
    pub standard_error: File,
    pub standard_input: Option<File>,
    pub inherited: Vec<IsolatedDescriptorBinding>,
}

impl IsolatedSpawnDescriptors {
    pub fn new(
        specification: &ProcessCommand,
        standard_output: &File,
        standard_error: &File,
        standard_input: Option<&File>,
        capability: Option<RawFd>,
    ) -> Result<Self> {
        let child_capability = safe_descriptor_minimum(specification)?;
        let minimum = child_capability
            .checked_add(NEXT_DESCRIPTOR_OFFSET)
            .context(DESCRIPTOR_RANGE_ERROR)?;
        let mut inherited = specification
            .inherited
            .iter()
            .map(|binding| {
                Ok(IsolatedDescriptorBinding {
                    source: duplicate(binding.file.as_raw_fd(), minimum)?,
                    child_descriptor: binding.child_descriptor,
                })
            })
            .collect::<Result<Vec<_>>>()?;
        if let Some(capability) = capability {
            inherited.push(IsolatedDescriptorBinding {
                source: duplicate(capability, minimum)?,
                child_descriptor: child_capability,
            });
        }
        Ok(Self {
            standard_output: duplicate(standard_output.as_raw_fd(), minimum)?,
            standard_error: duplicate(standard_error.as_raw_fd(), minimum)?,
            standard_input: standard_input
                .map(|source| duplicate(source.as_raw_fd(), minimum))
                .transpose()?,
            inherited,
        })
    }
}

fn safe_descriptor_minimum(specification: &ProcessCommand) -> Result<libc::c_int> {
    specification
        .inherited
        .iter()
        .map(|binding| binding.child_descriptor)
        .chain([libc::STDIN_FILENO, libc::STDOUT_FILENO, libc::STDERR_FILENO])
        .max()
        .and_then(|maximum| maximum.checked_add(NEXT_DESCRIPTOR_OFFSET))
        .context(DESCRIPTOR_RANGE_ERROR)
}

fn duplicate(source: RawFd, minimum: libc::c_int) -> Result<File> {
    let descriptor = unsafe { libc::fcntl(source, libc::F_DUPFD_CLOEXEC, minimum) };
    if descriptor == FCNTL_ERROR {
        return Err(std::io::Error::last_os_error()).context(DESCRIPTOR_DUPLICATION_CONTEXT);
    }
    Ok(unsafe { File::from_raw_fd(descriptor) })
}

#[cfg(all(test, feature = "yolo-unit-tests"))]
#[path = "../unit_tests/descriptors.rs"]
mod unit_tests;
