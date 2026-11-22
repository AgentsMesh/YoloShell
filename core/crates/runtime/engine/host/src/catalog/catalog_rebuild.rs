use super::catalog_durable_dir::DurableDirectory;
use super::catalog_durable_file::read_optional_durable_json;
use crate::constants::{
    RESUME_TRANSACTION_SCHEMA_VERSION, RUNTIME_RECORD_SCHEMA_VERSION,
    SESSION_MANIFEST_SCHEMA_VERSION, WORKER_LAUNCH_SCHEMA_VERSION,
};
use crate::model::{RuntimeRecord, TerminalManifest, WorkerLaunchSpec};
use crate::namespace_lease::NamespaceLease;
use crate::paths::Paths;
use anyhow::{Context, Result};
use std::collections::BTreeMap;
use std::ffi::OsStr;
use std::path::Path;
use uuid::Uuid;

const MANIFEST_LABEL: &str = "session manifest";
const RUNTIME_LABEL: &str = "runtime record";
const WORKER_LAUNCH_LABEL: &str = "worker launch";
const RESUME_TRANSACTION_LABEL: &str = "resume transaction";

struct DurableSession {
    directory: DurableDirectory,
    manifest: TerminalManifest,
}

pub(crate) fn durable_catalog_manifests(
    paths: &Paths,
    namespace: &NamespaceLease,
) -> Result<Vec<TerminalManifest>> {
    Ok(durable_sessions(paths, namespace)?
        .into_values()
        .map(|session| session.manifest)
        .collect())
}

pub(crate) fn durable_controller_generation(
    paths: &Paths,
    namespace: &NamespaceLease,
) -> Result<u64> {
    let mut floor = 0;
    for (terminal_id, session) in durable_sessions(paths, namespace)? {
        let runtime_path = paths.runtime_record(terminal_id);
        if let Some(record) = read_optional_durable_json::<RuntimeRecord>(
            &session.directory,
            file_name(&runtime_path)?,
            RUNTIME_LABEL,
        )? {
            if record.terminal_id == terminal_id
                && record.schema_version == RUNTIME_RECORD_SCHEMA_VERSION
            {
                floor = floor.max(record.controller_generation);
            } else {
                reject_generation_record(terminal_id, record.terminal_id, record.schema_version);
            }
        }
        let launch_path = paths.worker_launch(terminal_id);
        if let Some(launch) = read_optional_durable_json::<WorkerLaunchSpec>(
            &session.directory,
            file_name(&launch_path)?,
            WORKER_LAUNCH_LABEL,
        )? {
            if launch.terminal_id == terminal_id
                && launch.schema_version == WORKER_LAUNCH_SCHEMA_VERSION
            {
                floor = floor.max(launch.controller_generation);
            } else {
                reject_generation_record(terminal_id, launch.terminal_id, launch.schema_version);
            }
        }
        let transaction_path = paths.resume_transaction(terminal_id);
        if let Some(transaction) = read_optional_durable_json::<super::resume::ResumeTransaction>(
            &session.directory,
            file_name(&transaction_path)?,
            RESUME_TRANSACTION_LABEL,
        )? {
            if transaction.schema_version == RESUME_TRANSACTION_SCHEMA_VERSION
                && transaction.validate_for(terminal_id).is_ok()
            {
                floor = floor.max(transaction.launch.controller_generation);
            } else {
                reject_generation_record(
                    terminal_id,
                    transaction.terminal_id,
                    transaction.schema_version,
                );
            }
        }
    }
    Ok(floor)
}

fn durable_sessions(
    paths: &Paths,
    namespace: &NamespaceLease,
) -> Result<BTreeMap<Uuid, DurableSession>> {
    let sessions_path = paths.sessions_dir();
    let sessions = DurableDirectory::from_private(namespace.directory(&sessions_path)?);
    let mut durable = BTreeMap::new();
    for entry_name in sessions.entries()? {
        let Some(name) = entry_name.to_str() else {
            tracing::warn!(path = %sessions.entry_path(&entry_name).display(), "ignore non-UTF-8 session directory");
            continue;
        };
        let Ok(terminal_id) = Uuid::parse_str(name) else {
            continue;
        };
        if name != terminal_id.to_string() {
            tracing::warn!(directory = %name, %terminal_id, "ignore non-canonical session directory");
            continue;
        }
        let Some(directory) = sessions.open_directory(&entry_name)? else {
            tracing::warn!(%terminal_id, "ignore untrusted durable session directory");
            continue;
        };
        let manifest_path = paths.manifest(terminal_id);
        let Some(manifest) = read_optional_durable_json::<TerminalManifest>(
            &directory,
            file_name(&manifest_path)?,
            MANIFEST_LABEL,
        )?
        else {
            continue;
        };
        if manifest.terminal_id != terminal_id
            || manifest.schema_version != SESSION_MANIFEST_SCHEMA_VERSION
        {
            reject_manifest(terminal_id, &manifest);
            continue;
        }
        anyhow::ensure!(
            durable
                .insert(
                    terminal_id,
                    DurableSession {
                        directory,
                        manifest,
                    },
                )
                .is_none(),
            "duplicate canonical durable manifest {terminal_id}"
        );
    }
    Ok(durable)
}

fn file_name(path: &Path) -> Result<&OsStr> {
    path.file_name()
        .context("durable state path has no file name")
}

fn reject_manifest(expected: Uuid, manifest: &TerminalManifest) {
    tracing::error!(
        terminal_id = %expected,
        actual_session_id = %manifest.terminal_id,
        schema_version = manifest.schema_version,
        "durable manifest rejected during catalog rebuild"
    );
}

fn reject_generation_record(expected: Uuid, actual: Uuid, schema_version: u16) {
    tracing::error!(
        terminal_id = %expected,
        actual_session_id = %actual,
        schema_version,
        "durable generation record rejected during catalog rebuild"
    );
}
