use std::{
    fmt,
    path::{Path, PathBuf},
};

/// A durable read observed different file versions. This is never successful
/// data, and callers must revalidate their own authority before any retry.
#[derive(Debug)]
pub struct ReadVersionChanged(VersionChange);

#[derive(Debug)]
enum VersionChange {
    State(PathBuf),
    PrivateFile,
}

impl ReadVersionChanged {
    pub(crate) fn state(path: &Path) -> Self {
        Self(VersionChange::State(path.to_owned()))
    }

    pub(crate) fn private_file() -> Self {
        Self(VersionChange::PrivateFile)
    }
}

impl fmt::Display for ReadVersionChanged {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.0 {
            VersionChange::State(path) => {
                write!(formatter, "state changed while reading: {}", path.display())
            }
            VersionChange::PrivateFile => formatter.write_str("private file version changed"),
        }
    }
}

impl std::error::Error for ReadVersionChanged {}
