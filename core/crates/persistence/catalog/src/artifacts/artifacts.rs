use super::*;
use std::ffi::OsString;
use std::fs;
use std::path::PathBuf;

const EXPECTED_LINK_COUNT: u64 = 1;
const SQLITE_JOURNAL_SUFFIX: &str = "-journal";
const SQLITE_SHM_SUFFIX: &str = "-shm";
const SQLITE_WAL_SUFFIX: &str = "-wal";

pub(super) struct CatalogPathSnapshot {
    artifacts: Vec<ArtifactSnapshot>,
}

struct ArtifactSnapshot {
    path: PathBuf,
    identity: Option<FileIdentity>,
    may_be_created: bool,
}

#[cfg(unix)]
#[derive(Clone, Copy, PartialEq, Eq)]
struct FileIdentity {
    device: u64,
    inode: u64,
}

#[cfg(not(unix))]
#[derive(Clone, Copy, PartialEq, Eq)]
struct FileIdentity;

pub(super) fn validate_catalog_path(path: &Path) -> Result<CatalogPathSnapshot> {
    let mut paths = catalog_artifacts(path).into_iter();
    let main = paths.next().context("catalog artifact set is empty")?;
    let mut artifacts = vec![ArtifactSnapshot {
        identity: inspect_artifact(&main)?,
        path: main,
        may_be_created: true,
    }];
    for sidecar in paths {
        artifacts.push(ArtifactSnapshot {
            identity: inspect_artifact(&sidecar)?,
            path: sidecar,
            may_be_created: false,
        });
    }
    Ok(CatalogPathSnapshot { artifacts })
}

pub(super) fn revalidate_catalog_path(snapshot: &CatalogPathSnapshot) -> Result<()> {
    for artifact in &snapshot.artifacts {
        let current = inspect_artifact(&artifact.path)?;
        if artifact.identity.is_none() && artifact.may_be_created {
            if current.is_none() {
                return Err(super::rebuild_reason::unsafe_artifact(
                    "SQLite did not create the catalog file",
                ));
            }
        } else if current != artifact.identity {
            return Err(super::rebuild_reason::unsafe_artifact(format!(
                "catalog artifact changed while opening: {}",
                artifact.path.display()
            )));
        }
    }
    Ok(())
}

pub(super) fn sqlite_catalog_path(path: &Path) -> Result<PathBuf> {
    let parent = path.parent().context("catalog has no parent directory")?;
    let file_name = path.file_name().context("catalog has no file name")?;
    Ok(parent
        .canonicalize()
        .with_context(|| format!("canonicalize catalog parent {}", parent.display()))?
        .join(file_name))
}

pub(super) fn quarantine_catalog(path: &Path) -> Result<()> {
    super::artifacts_quarantine::quarantine_catalog(path)
}

fn inspect_artifact(path: &Path) -> Result<Option<FileIdentity>> {
    match fs::symlink_metadata(path) {
        Ok(metadata) => {
            if !metadata.file_type().is_file() {
                return Err(super::rebuild_reason::unsafe_artifact(format!(
                    "catalog artifact is not a regular file: {}",
                    path.display()
                )));
            }
            #[cfg(unix)]
            {
                use std::os::unix::fs::MetadataExt;
                if metadata.nlink() != EXPECTED_LINK_COUNT {
                    return Err(super::rebuild_reason::unsafe_artifact(format!(
                        "catalog artifact has unexpected hard links: {}",
                        path.display()
                    )));
                }
                Ok(Some(FileIdentity {
                    device: metadata.dev(),
                    inode: metadata.ino(),
                }))
            }
            #[cfg(not(unix))]
            Ok(Some(FileIdentity))
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => {
            Err(error).with_context(|| format!("inspect catalog artifact {}", path.display()))
        }
    }
}

pub(super) fn catalog_artifacts(path: &Path) -> [PathBuf; 4] {
    [
        path.to_path_buf(),
        sqlite_sidecar(path, SQLITE_JOURNAL_SUFFIX),
        sqlite_sidecar(path, SQLITE_WAL_SUFFIX),
        sqlite_sidecar(path, SQLITE_SHM_SUFFIX),
    ]
}

fn sqlite_sidecar(path: &Path, suffix: &str) -> PathBuf {
    let mut name: OsString = path.as_os_str().to_owned();
    name.push(suffix);
    PathBuf::from(name)
}
