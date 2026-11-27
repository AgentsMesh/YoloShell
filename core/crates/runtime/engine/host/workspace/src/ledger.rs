//! Durable worktree operation ledger. Records are evidence for restart
//! reconciliation; they never authorize filesystem deletion.

use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::fs::OpenOptions;
use std::io::Write;
use std::os::unix::fs::OpenOptionsExt;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

pub(crate) const WORKTREE_LEDGER_FILE: &str = "worktree-ledger.jsonl";
const MAX_LEDGER_RECORDS: usize = 1024;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum Operation {
    Create,
    Remove,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum Phase {
    Reserved,
    Linked,
    Removed,
    Failed,
    Unknown,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct LedgerRecord {
    pub token: String,
    pub operation: Operation,
    pub root: String,
    pub target: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub branch: Option<String>,
    pub phase: Phase,
    pub at_ms: i64,
}

pub(crate) struct WorktreeLedger {
    path: PathBuf,
    records: Mutex<VecDeque<LedgerRecord>>,
}

impl WorktreeLedger {
    pub(crate) fn open(path: PathBuf) -> Result<Self, std::io::Error> {
        let mut records = read(&path)?.into_iter().collect::<VecDeque<_>>();
        while records.len() > MAX_LEDGER_RECORDS {
            records.pop_front();
        }
        Ok(Self {
            path,
            records: Mutex::new(records),
        })
    }

    pub(crate) fn reserve(
        &self,
        token: String,
        operation: Operation,
        root: &Path,
        target: &Path,
        branch: Option<String>,
    ) -> Result<(), std::io::Error> {
        let record = LedgerRecord {
            token,
            operation,
            root: root.display().to_string(),
            target: target.display().to_string(),
            branch,
            phase: Phase::Reserved,
            at_ms: now_ms(),
        };
        self.append(record)
    }

    pub(crate) fn finish(&self, token: &str, phase: Phase) -> Result<(), std::io::Error> {
        let mut state = self
            .records
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let mut record = state
            .iter()
            .rev()
            .find(|record| record.token == token)
            .cloned()
            .ok_or_else(|| std::io::Error::other("worktree ledger token is missing"))?;
        record.phase = phase;
        record.at_ms = now_ms();
        append_record(&self.path, &record)?;
        state.push_back(record);
        if state.len() > MAX_LEDGER_RECORDS {
            state.pop_front();
            rewrite(&self.path, &state)?;
        }
        Ok(())
    }

    /// Latest unfinished create/remove attempt for every token.
    pub(crate) fn pending(&self) -> Vec<LedgerRecord> {
        let mut latest = std::collections::BTreeMap::new();
        for record in self
            .records
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .iter()
        {
            latest.insert(record.token.clone(), record.clone());
        }
        latest
            .into_values()
            .filter(|record| matches!(record.phase, Phase::Reserved | Phase::Unknown))
            .collect()
    }

    fn append(&self, record: LedgerRecord) -> Result<(), std::io::Error> {
        let mut state = self
            .records
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        append_record(&self.path, &record)?;
        state.push_back(record);
        if state.len() > MAX_LEDGER_RECORDS {
            state.pop_front();
            rewrite(&self.path, &state)?;
        }
        Ok(())
    }
}

fn read(path: &Path) -> Result<Vec<LedgerRecord>, std::io::Error> {
    let Ok(bytes) = std::fs::read(path) else {
        return Ok(Vec::new());
    };
    Ok(String::from_utf8_lossy(&bytes)
        .lines()
        .filter_map(|line| serde_json::from_str::<LedgerRecord>(line).ok())
        .collect())
}

fn append_record(path: &Path, record: &LedgerRecord) -> Result<(), std::io::Error> {
    let mut body = serde_json::to_vec(record).map_err(std::io::Error::other)?;
    body.push(b'\n');
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .mode(0o600)
        .open(path)?;
    file.write_all(&body)
}

fn rewrite(path: &Path, records: &VecDeque<LedgerRecord>) -> Result<(), std::io::Error> {
    let temporary = path.with_extension("jsonl.tmp");
    let mut body = Vec::new();
    for record in records {
        body.extend(serde_json::to_vec(record).map_err(std::io::Error::other)?);
        body.push(b'\n');
    }
    let mut file = OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .mode(0o600)
        .open(&temporary)?;
    file.write_all(&body)?;
    file.sync_all()?;
    std::fs::rename(temporary, path)
}

fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis() as i64)
        .unwrap_or_default()
}
