use super::*;
use anyhow::{Context, Result, ensure};
use yolo_process_test_support::{BootIdentityRelation, boot_id, boot_identity_relation};
use yolo_runtime::constants::RUNTIME_RECORD_SCHEMA_VERSION;

const READ_SESSIONS_CONTEXT: &str = "read runtime cleanup session directory";
const READ_ENTRY_CONTEXT: &str = "read runtime cleanup session entry";
const READ_RECORD_CONTEXT: &str = "read runtime cleanup record";
const UNTRUSTED_SESSION_ENTRY_DETAIL: &str = "runtime cleanup session entry is not a directory";
const RECORD_SESSION_MISMATCH_DETAIL: &str =
    "runtime cleanup record is in another session directory";
const RECORD_SCHEMA_MISMATCH_DETAIL: &str = "runtime cleanup record schema is unsupported";
const RECORD_SOCKET_MISMATCH_DETAIL: &str =
    "runtime cleanup record belongs to another socket namespace";
const RECORD_BOOT_MISMATCH_DETAIL: &str = "runtime cleanup record belongs to another boot";
const EXPECTED_BOOT_UNTRUSTED_DETAIL: &str =
    "runtime cleanup expected boot identity is not authoritative";

pub(super) struct RuntimeRecordScan {
    pub(super) records: Vec<RuntimeRecord>,
    pub(super) failures: Vec<anyhow::Error>,
}

pub(super) fn readable_runtime_records(paths: &Paths) -> RuntimeRecordScan {
    let expected_boot_id = boot_id();
    readable_runtime_records_for_boot(paths, &expected_boot_id)
}

pub(super) fn readable_runtime_records_for_boot(
    paths: &Paths,
    expected_boot_id: &str,
) -> RuntimeRecordScan {
    if let Err(error) = validate_expected_boot_id(expected_boot_id) {
        return RuntimeRecordScan {
            records: Vec::new(),
            failures: vec![error],
        };
    }
    let entries = match std::fs::read_dir(paths.sessions_dir()) {
        Ok(entries) => entries,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return empty_scan(),
        Err(error) => {
            return RuntimeRecordScan {
                records: Vec::new(),
                failures: vec![anyhow::Error::new(error).context(READ_SESSIONS_CONTEXT)],
            };
        }
    };
    let mut scan = empty_scan();
    for entry in entries {
        match read_entry(paths, entry, expected_boot_id) {
            Ok(Some(record)) => scan.records.push(record),
            Ok(None) => {}
            Err(error) => scan.failures.push(error),
        }
    }
    scan
}

fn empty_scan() -> RuntimeRecordScan {
    RuntimeRecordScan {
        records: Vec::new(),
        failures: Vec::new(),
    }
}

fn read_entry(
    paths: &Paths,
    entry: std::io::Result<std::fs::DirEntry>,
    expected_boot_id: &str,
) -> Result<Option<RuntimeRecord>> {
    let entry = entry.context(READ_ENTRY_CONTEXT)?;
    let Some(name) = entry.file_name().to_str().map(str::to_owned) else {
        return Ok(None);
    };
    let Ok(terminal_id) = Uuid::parse_str(&name) else {
        return Ok(None);
    };
    if name != terminal_id.to_string() {
        return Ok(None);
    }
    let file_type = entry.file_type().context(READ_ENTRY_CONTEXT)?;
    ensure!(
        file_type.is_dir(),
        "{UNTRUSTED_SESSION_ENTRY_DETAIL}: {}",
        entry.path().display()
    );
    let record_path = paths.runtime_record(terminal_id);
    if !record_path.try_exists().context(READ_RECORD_CONTEXT)? {
        return Ok(None);
    }
    let record = read_json(&record_path)
        .with_context(|| format!("{READ_RECORD_CONTEXT}: {}", record_path.display()))?;
    validate_runtime_record_for_boot(paths, &record, Some(terminal_id), expected_boot_id)?;
    Ok(Some(record))
}

pub(super) fn validate_runtime_record(
    paths: &Paths,
    record: &RuntimeRecord,
    directory_session: Option<Uuid>,
) -> Result<()> {
    let expected_boot_id = boot_id();
    validate_runtime_record_for_boot(paths, record, directory_session, &expected_boot_id)
}

pub(super) fn validate_runtime_record_for_boot(
    paths: &Paths,
    record: &RuntimeRecord,
    directory_session: Option<Uuid>,
    expected_boot_id: &str,
) -> Result<()> {
    validate_expected_boot_id(expected_boot_id)?;
    ensure!(
        directory_session.is_none_or(|terminal_id| terminal_id == record.terminal_id),
        "{RECORD_SESSION_MISMATCH_DETAIL}: {}",
        record.terminal_id
    );
    ensure!(
        record.schema_version == RUNTIME_RECORD_SCHEMA_VERSION,
        "{RECORD_SCHEMA_MISMATCH_DETAIL}: {}",
        record.schema_version
    );
    ensure!(
        record.socket_path == paths.worker_socket(record.terminal_id)?,
        "{RECORD_SOCKET_MISMATCH_DETAIL}: {}",
        record.socket_path.display()
    );
    ensure!(
        boot_identity_relation(&record.boot_id, expected_boot_id) == BootIdentityRelation::Same,
        "{RECORD_BOOT_MISMATCH_DETAIL}: {}",
        record.boot_id
    );
    Ok(())
}

fn validate_expected_boot_id(expected_boot_id: &str) -> Result<()> {
    ensure!(
        boot_identity_relation(expected_boot_id, expected_boot_id) == BootIdentityRelation::Same,
        "{EXPECTED_BOOT_UNTRUSTED_DETAIL}: {expected_boot_id}"
    );
    Ok(())
}
