use super::evidence_path::AnchoredEvidence;
use super::evidence_reader;
use super::{CodexAdapter, constants};
use anyhow::Result;
use serde_json::Value;
use std::fs::File;
use std::io::SeekFrom;
use std::path::Path;

const MAX_TURN_ID_BYTES: usize = 256;

/// A provider turn-journal record that ends a turn. `task_complete` is written
/// only after the Stop hook result is final; `turn_aborted` is written for an
/// interrupted turn and no Stop hook is dispatched.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TurnEndKind {
    Completed,
    Aborted,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TurnEnd {
    pub turn_id: String,
    pub kind: TurnEndKind,
}

impl CodexAdapter {
    /// Read the latest turn-ending record from the pinned rollout transcript.
    /// The parent directory chain, ownership and hard-link identity are
    /// verified before and after the read.
    pub fn read_turn_end(&self, path: &Path, data_home: &Path) -> Result<Option<TurnEnd>> {
        let Some(mut evidence) = AnchoredEvidence::open(data_home, path)? else {
            return Ok(None);
        };
        let result = read_tail(evidence.file_mut())?;
        if !evidence.verify()? {
            return Ok(None);
        }
        Ok(result)
    }
}

fn read_tail(file: &mut File) -> Result<Option<TurnEnd>> {
    let size = file.metadata()?.len();
    if size == 0 {
        return Ok(None);
    }
    let start = size.saturating_sub(constants::TURN_TAIL_BYTES);
    let bytes = evidence_reader::read_at(file, SeekFrom::Start(start), constants::TURN_TAIL_BYTES)?;
    let bytes = if start == 0 {
        bytes.as_slice()
    } else {
        let Some(end) = bytes.iter().position(|byte| *byte == b'\n') else {
            return Ok(None);
        };
        &bytes[end + 1..]
    };
    let mut latest: Option<String> = None;
    let mut terminal: Option<TurnEndKind> = None;
    for line in bytes
        .split(|byte| *byte == b'\n')
        .filter(|line| !line.is_empty())
    {
        let Ok(record) = serde_json::from_slice::<Value>(line) else {
            continue;
        };
        if record
            .get(constants::RECORD_TYPE_KEY)
            .and_then(Value::as_str)
            != Some(constants::EVENT_MESSAGE_TYPE)
        {
            continue;
        }
        let Some(payload) = record.get(constants::PAYLOAD_KEY) else {
            continue;
        };
        if payload.get(constants::AGENT_ID_KEY).is_some() {
            continue;
        }
        let Some(turn_id) = payload.get(constants::TURN_ID_KEY).and_then(Value::as_str) else {
            continue;
        };
        if turn_id.is_empty() || turn_id.len() > MAX_TURN_ID_BYTES {
            continue;
        }
        let kind = payload
            .get(constants::EVENT_KIND_KEY)
            .and_then(Value::as_str);
        let ending = match kind {
            Some(constants::TASK_COMPLETE_EVENT) => Some(TurnEndKind::Completed),
            Some(constants::TURN_ABORTED_EVENT) => Some(TurnEndKind::Aborted),
            _ => None,
        };
        if latest.as_deref() == Some(turn_id) {
            if let Some(ending) = ending {
                terminal = Some(ending);
            } else if kind == Some(constants::TASK_STARTED_EVENT) {
                terminal = None;
            }
        } else {
            latest = Some(turn_id.to_owned());
            terminal = ending;
        }
    }
    Ok(latest
        .zip(terminal)
        .map(|(turn_id, kind)| TurnEnd { turn_id, kind }))
}
