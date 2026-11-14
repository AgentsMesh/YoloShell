use super::super::ObservedProcess;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::ffi::{OsStr, OsString};
use std::os::unix::ffi::{OsStrExt, OsStringExt};
use std::path::{Path, PathBuf};

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct ProcessSnapshotEnvelope {
    pub(super) schema_version: u16,
    pub(super) root_pid: u32,
    pub(super) root_start_time: u64,
    pub(super) started_at_ms: i64,
    pub(super) completed_at_ms: i64,
    pub(super) outcome: ProcessSnapshotWireOutcome,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(tag = "status", rename_all = "snake_case", deny_unknown_fields)]
pub(super) enum ProcessSnapshotWireOutcome {
    Stable {
        #[serde(deserialize_with = "super::bounded::processes")]
        processes: Vec<ObservedProcessWire>,
    },
    RootChanged,
    Failed {
        detail: String,
    },
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct ObservedProcessWire {
    pub(super) pid: u32,
    pub(super) start_time: u64,
    pub(super) parent_pid: u32,
    pub(super) process_group: i32,
    pub(super) foreground_group: i32,
    pub(super) executable: String,
    #[serde(deserialize_with = "super::bounded::argv")]
    pub(super) argv: Vec<String>,
    #[serde(deserialize_with = "super::bounded::environment")]
    pub(super) environment: Option<BTreeMap<String, String>>,
    pub(super) cwd: Option<UnixPathWire>,
    #[serde(deserialize_with = "super::bounded::evidence")]
    pub(super) evidence_paths: Option<Vec<UnixPathWire>>,
}

#[derive(Debug, Deserialize, Serialize)]
pub(super) struct UnixPathWire(#[serde(with = "serde_bytes")] Vec<u8>);

impl UnixPathWire {
    pub(super) fn byte_len(&self) -> usize {
        self.0.len()
    }

    pub(super) fn is_absolute(&self) -> bool {
        Path::new(OsStr::from_bytes(&self.0)).is_absolute()
    }

    pub(super) fn contains_nul(&self) -> bool {
        self.0.contains(&u8::default())
    }
}

impl From<PathBuf> for UnixPathWire {
    fn from(path: PathBuf) -> Self {
        Self(path.as_os_str().as_bytes().to_vec())
    }
}

impl From<UnixPathWire> for PathBuf {
    fn from(path: UnixPathWire) -> Self {
        OsString::from_vec(path.0).into()
    }
}

impl From<ObservedProcess> for ObservedProcessWire {
    fn from(process: ObservedProcess) -> Self {
        Self {
            pid: process.pid,
            start_time: process.start_time,
            parent_pid: process.parent_pid,
            process_group: process.process_group,
            foreground_group: process.foreground_group,
            executable: process.executable,
            argv: process.argv,
            environment: process.environment,
            cwd: process.cwd.map(Into::into),
            evidence_paths: process
                .evidence_paths
                .map(|paths| paths.into_iter().map(Into::into).collect()),
        }
    }
}

impl From<ObservedProcessWire> for ObservedProcess {
    fn from(process: ObservedProcessWire) -> Self {
        Self {
            pid: process.pid,
            start_time: process.start_time,
            parent_pid: process.parent_pid,
            process_group: process.process_group,
            foreground_group: process.foreground_group,
            executable: process.executable,
            argv: process.argv,
            environment: process.environment,
            cwd: process.cwd.map(Into::into),
            evidence_paths: process
                .evidence_paths
                .map(|paths| paths.into_iter().map(Into::into).collect()),
        }
    }
}
