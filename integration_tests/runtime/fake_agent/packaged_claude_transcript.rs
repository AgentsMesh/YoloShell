use crate::fake_agent_constants::*;
use serde_json::{Value, json};
use std::error::Error;
use std::ffi::OsString;
use std::fs::{File, OpenOptions};
use std::io::{BufWriter, Write};
use std::os::unix::fs::OpenOptionsExt;
use std::path::{Component, Path, PathBuf};
use yolo_runtime::agent::claude::constants as claude;

pub(crate) struct Transcript {
    path: PathBuf,
    writer: BufWriter<File>,
}

impl Transcript {
    pub(crate) fn open(
        data_home: &Path,
        session_id: &str,
        cwd: &Path,
    ) -> Result<Self, Box<dyn Error>> {
        let directory = data_home
            .join(claude::EVIDENCE_DIR)
            .join(project_directory()?);
        std::fs::create_dir_all(&directory)?;
        let path = directory
            .join(session_id)
            .with_extension(yolo_runtime::agent::constants::JSONL_EXTENSION);
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .mode(PACKAGED_PRIVATE_MODE)
            .open(&path)?;
        let mut transcript = Self {
            path,
            writer: BufWriter::new(file),
        };
        transcript.append(waiting_record(session_id, cwd))?;
        Ok(transcript)
    }

    pub(crate) fn path(&self) -> &Path {
        &self.path
    }
    pub(crate) fn chat_message(
        &mut self,
        session: &str,
        cwd: &Path,
        role: &str,
        text: &str,
    ) -> Result<(), Box<dyn Error>> {
        let mut record = common(session, cwd, role, &uuid::Uuid::new_v4().to_string());
        record[crate::chat_io::constants::MESSAGE] = json!({
            (crate::chat_io::constants::ROLE): role, (claude::CONTENT_KEY): text,
        });
        self.append(record)
    }

    pub(crate) fn running(&mut self, session_id: &str, cwd: &Path) -> Result<(), Box<dyn Error>> {
        self.append(running_record(session_id, cwd))
    }

    pub(crate) fn waiting(&mut self, session_id: &str, cwd: &Path) -> Result<(), Box<dyn Error>> {
        self.append(completion_record(session_id, cwd))
    }

    fn append(&mut self, value: Value) -> Result<(), Box<dyn Error>> {
        serde_json::to_writer(&mut self.writer, &value)?;
        self.writer
            .write_all(&[crate::fake_agent_constants::INVOCATION_LINE_ENDING])?;
        self.writer.flush()?;
        self.writer.get_ref().sync_data()?;
        Ok(())
    }
}

fn project_directory() -> Result<PathBuf, Box<dyn Error>> {
    Ok(project_directory_from(std::env::var_os(
        PACKAGED_PROJECT_DIRECTORY_ENV,
    ))?)
}

pub(crate) fn project_directory_from(value: Option<OsString>) -> Result<PathBuf, &'static str> {
    let path = PathBuf::from(value.unwrap_or_else(|| OsString::from(PACKAGED_PROJECT_DIRECTORY)));
    let mut components = path.components();
    match (components.next(), components.next()) {
        (Some(Component::Normal(_)), None) => Ok(path),
        _ => Err(PACKAGED_PROJECT_DIRECTORY_INVALID),
    }
}

fn common(session_id: &str, cwd: &Path, record_type: &str, uuid: &str) -> Value {
    json!({
        (PACKAGED_PARENT_UUID_FIELD): null,
        (PACKAGED_SIDECHAIN_FIELD): false,
        (PACKAGED_USER_TYPE_FIELD): PACKAGED_REDACTED_USER_TYPE,
        (claude::CWD_KEY): cwd,
        (claude::SESSION_ID_KEY): session_id,
        (PACKAGED_VERSION_FIELD): PACKAGED_TRANSCRIPT_VERSION,
        (PACKAGED_GIT_BRANCH_FIELD): PACKAGED_REDACTED_BRANCH,
        (claude::RECORD_TYPE_KEY): record_type,
        (PACKAGED_UUID_FIELD): uuid,
        (PACKAGED_TIMESTAMP_FIELD): PACKAGED_TIMESTAMP,
    })
}

fn waiting_record(session_id: &str, cwd: &Path) -> Value {
    let mut record = common(session_id, cwd, claude::SYSTEM_RECORD, PACKAGED_SYSTEM_UUID);
    if let Some(fields) = record.as_object_mut() {
        fields.insert(
            claude::SUBTYPE_KEY.into(),
            claude::TURN_DURATION_SUBTYPE.into(),
        );
        fields.insert(PACKAGED_DURATION_FIELD.into(), 0.into());
        fields.insert(PACKAGED_MESSAGE_COUNT_FIELD.into(), 0.into());
    }
    record
}

fn running_record(session_id: &str, cwd: &Path) -> Value {
    let mut record = common(session_id, cwd, claude::USER_RECORD, PACKAGED_USER_UUID);
    if let Some(fields) = record.as_object_mut() {
        fields.insert(
            claude::MESSAGE_KEY.into(),
            json!({
                (PACKAGED_ROLE_FIELD): PACKAGED_REDACTED_ROLE,
                (claude::CONTENT_KEY): PACKAGED_REDACTED_USER_CONTENT,
            }),
        );
    }
    record
}

fn completion_record(session_id: &str, cwd: &Path) -> Value {
    let mut record = common(
        session_id,
        cwd,
        claude::ASSISTANT_RECORD,
        PACKAGED_ASSISTANT_UUID,
    );
    if let Some(fields) = record.as_object_mut() {
        fields.insert(
            claude::MESSAGE_KEY.into(),
            json!({
                (PACKAGED_MESSAGE_ID_FIELD): PACKAGED_ASSISTANT_MESSAGE_ID,
                (PACKAGED_MESSAGE_TYPE_FIELD): PACKAGED_REDACTED_MESSAGE_TYPE,
                (PACKAGED_ROLE_FIELD): PACKAGED_REDACTED_ROLE,
                (PACKAGED_MODEL_FIELD): PACKAGED_REDACTED_MODEL,
                (claude::CONTENT_KEY): [{
                    (claude::CONTENT_TYPE_KEY): claude::TEXT_CONTENT,
                    (claude::TEXT_VALUE_KEY): PACKAGED_REDACTED_ASSISTANT_CONTENT,
                }],
                (claude::STOP_REASON_KEY): claude::END_TURN_REASON,
                (PACKAGED_STOP_SEQUENCE_FIELD): null,
                (claude::CONTEXT_MANAGEMENT_KEY): {},
            }),
        );
    }
    record
}
