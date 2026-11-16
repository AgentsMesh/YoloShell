use super::{BrokerPaths, BrokerStateAnchor};
use crate::paths::directory::PrivateDirectory;
use crate::paths::directory_stream::DirectoryStream;
use crate::paths::private_file::PrivateFileEntry;
use anyhow::Result;
use serde::Serialize;
use serde::de::DeserializeOwned;
use std::ffi::CString;
use std::path::{Component, Path, PathBuf};

const INVALID_ENTRY_NAME: &str = "invalid broker supervision entry name";

#[derive(Clone, Debug)]
pub struct BrokerSupervisionStore {
    paths: BrokerPaths,
}

impl BrokerSupervisionStore {
    pub fn open_expected(
        state_dir: impl Into<PathBuf>,
        expected: BrokerStateAnchor,
    ) -> Result<Self> {
        let paths = BrokerPaths::new_expected(state_dir, expected)?;
        paths.verify_state()?;
        Ok(Self { paths })
    }

    pub fn write<T: Serialize>(&self, name: &str, value: &T) -> Result<()> {
        self.paths.verify_state()?;
        crate::paths::atomic_write_entry_guarded(self.entry(name)?, value, || {
            self.paths.verify_state()
        })?;
        self.paths.verify_state()
    }

    pub fn read<T: DeserializeOwned>(&self, name: &str) -> Result<T> {
        self.paths.verify_state()?;
        let value =
            crate::paths::read_entry_guarded(self.entry(name)?, || self.paths.verify_state())?;
        self.paths.verify_state()?;
        Ok(value)
    }

    pub fn remove(&self, name: &str) -> Result<()> {
        self.paths.verify_state()?;
        self.entry(name)?.remove()?;
        self.paths.verify_state()
    }

    pub fn visit_names_with_prefix(
        &self,
        prefix: &str,
        mut visit: impl FnMut(String),
    ) -> Result<()> {
        self.paths.verify_state()?;
        let directory = self.state_directory()?;
        let mut stream = DirectoryStream::open(directory.descriptor())?;
        while let Some(name) = stream.next_name()? {
            let Ok(name) = name.into_string() else {
                continue;
            };
            if name.starts_with(prefix) {
                visit(name);
            }
        }
        self.paths.verify_state()
    }

    pub fn remove_inert_entries(&self, names: &[String]) -> Result<()> {
        if names.is_empty() {
            return Ok(());
        }
        self.paths.verify_state()?;
        let names = names
            .iter()
            .map(|name| {
                validate_name(name)?;
                CString::new(name.as_bytes()).map_err(Into::into)
            })
            .collect::<Result<Vec<_>>>()?;
        let directory = self.state_directory()?;
        for name in names {
            let result = unsafe { libc::unlinkat(directory.descriptor(), name.as_ptr(), 0) };
            if result != 0 {
                let error = std::io::Error::last_os_error();
                if error.kind() != std::io::ErrorKind::NotFound {
                    return Err(error.into());
                }
            }
        }
        directory.sync()?;
        self.paths.verify_state()
    }

    pub fn contain_recorded_broker(
        &self,
        name: &str,
        authority: super::BrokerLaunchAuthority,
    ) -> Result<()> {
        super::supervision_containment::contain(self, name, authority)
    }

    pub(super) fn verify_state(&self) -> Result<()> {
        self.paths.verify_state()
    }

    fn entry(&self, name: &str) -> Result<PrivateFileEntry> {
        validate_name(name)?;
        let directory = self.state_directory()?;
        let path = self.paths.state_dir().join(name);
        PrivateFileEntry::from_directory(directory, &path)
    }

    fn state_directory(&self) -> Result<PrivateDirectory> {
        self.paths.state_directory()
    }
}

fn validate_name(name: &str) -> Result<()> {
    let mut components = Path::new(name).components();
    anyhow::ensure!(
        matches!(components.next(), Some(Component::Normal(_))) && components.next().is_none(),
        INVALID_ENTRY_NAME
    );
    Ok(())
}

#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-unit-tests"))]
#[path = "../../unit_tests/supervision_store_tests.rs"]
mod tests;
