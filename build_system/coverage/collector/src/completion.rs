use crate::constants::*;
use std::fs::{File, Metadata, OpenOptions};
use std::io::{self, Write};
use std::os::unix::fs::{MetadataExt, OpenOptionsExt};
use std::path::{Path, PathBuf};

pub(crate) struct Completion(Option<OwnedMarker>);

struct OwnedMarker {
    path: PathBuf,
    file: File,
    metadata: Metadata,
}

impl Completion {
    pub(crate) fn capture() -> io::Result<Self> {
        let path = std::env::var_os(PREMATURE_EXIT_ENV).map(PathBuf::from);
        Self::create(path.as_deref())
    }

    pub(crate) fn create(path: Option<&Path>) -> io::Result<Self> {
        let Some(path) = path else {
            return Ok(Self(None));
        };
        let mut file = match OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(COMPLETION_MODE)
            .open(path)
        {
            Ok(file) => file,
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => return Ok(Self(None)),
            Err(error) => return Err(error),
        };
        file.write_all(INCOMPLETE_COVERAGE.as_bytes())?;
        let metadata = file.metadata()?;
        Ok(Self(Some(OwnedMarker {
            path: path.to_owned(),
            file,
            metadata,
        })))
    }

    pub(crate) fn finish(self) -> io::Result<()> {
        let Some(marker) = self.0 else {
            return Ok(());
        };
        let current = std::fs::symlink_metadata(&marker.path)?;
        let retained = marker.file.metadata()?;
        if !same_marker(&marker.metadata, &current) || !same_marker(&marker.metadata, &retained) {
            return Err(io::Error::other(COMPLETION_CHANGED));
        }
        std::fs::remove_file(marker.path)
    }
}

fn same_marker(expected: &Metadata, current: &Metadata) -> bool {
    current.is_file()
        && current.nlink() == COMPLETION_LINKS
        && current.dev() == expected.dev()
        && current.ino() == expected.ino()
        && current.mode() == expected.mode()
        && current.len() == expected.len()
        && current.ctime() == expected.ctime()
        && current.ctime_nsec() == expected.ctime_nsec()
        && current.mtime() == expected.mtime()
        && current.mtime_nsec() == expected.mtime_nsec()
}
