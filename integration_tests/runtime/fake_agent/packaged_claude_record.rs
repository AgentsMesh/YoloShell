use crate::fake_agent_constants::*;
use serde_json::json;
use std::error::Error;
use std::fs::OpenOptions;
use std::io::{BufWriter, Write};
use std::os::unix::fs::OpenOptionsExt;
use std::path::Path;

const RECORD_DIRECTORY_ERROR: &str = "packaged Claude record directory is unavailable";
const INVOCATION_MODE_ERROR: &str = "packaged Claude invocation mode is invalid";

pub(crate) struct InvocationRecord<'a> {
    pub(crate) mode: &'a str,
    pub(crate) session_id: &'a str,
    pub(crate) arguments: &'a [String],
    pub(crate) hook_count: usize,
    pub(crate) cwd: &'a Path,
    pub(crate) data_home: &'a Path,
    pub(crate) transcript: &'a Path,
}

pub(crate) fn write(record: InvocationRecord<'_>) -> Result<(), Box<dyn Error>> {
    let directory = std::env::var_os(PACKAGED_RECORD_DIRECTORY_ENV)
        .map(std::path::PathBuf::from)
        .ok_or(RECORD_DIRECTORY_ERROR)?;
    let runtime_epoch = std::env::var(yolo_runtime::agent::constants::YOLO_RUNTIME_EPOCH_ENV)?;
    write_at(&directory, &runtime_epoch, record)
}

pub(crate) fn write_at(
    directory: &Path,
    runtime_epoch: &str,
    record: InvocationRecord<'_>,
) -> Result<(), Box<dyn Error>> {
    std::fs::create_dir_all(directory)?;
    let name = match record.mode {
        PACKAGED_FRESH_MODE => PACKAGED_FRESH_RECORD,
        PACKAGED_RESUME_MODE => PACKAGED_RESUME_RECORD,
        _ => return Err(INVOCATION_MODE_ERROR.into()),
    };
    let file = OpenOptions::new()
        .create_new(true)
        .write(true)
        .mode(PACKAGED_PRIVATE_MODE)
        .open(directory.join(name))?;
    let mut writer = BufWriter::new(file);
    serde_json::to_writer(
        &mut writer,
        &json!({
            (PACKAGED_RECORD_SCHEMA_FIELD): PACKAGED_RECORD_SCHEMA_VERSION,
            (PACKAGED_RECORD_MODE_FIELD): record.mode,
            (PACKAGED_RECORD_SESSION_FIELD): record.session_id,
            (PACKAGED_RECORD_RUNTIME_FIELD): runtime_epoch,
            (PACKAGED_RECORD_ARGUMENTS_FIELD): record.arguments,
            (PACKAGED_RECORD_HOOK_COUNT_FIELD): record.hook_count,
            (PACKAGED_RECORD_PROCESS_FIELD): std::process::id(),
            (PACKAGED_RECORD_CWD_FIELD): record.cwd,
            (PACKAGED_RECORD_DATA_HOME_FIELD): record.data_home,
            (PACKAGED_RECORD_TRANSCRIPT_FIELD): record.transcript,
        }),
    )?;
    writer.write_all(&[INVOCATION_LINE_ENDING])?;
    writer.flush()?;
    writer.get_ref().sync_all()?;
    Ok(())
}
