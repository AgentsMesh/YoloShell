use super::contract::{
    HISTORICAL_WITNESS_ACKNOWLEDGEMENT_ENVIRONMENT, HISTORICAL_WITNESS_IDENTITY_ENVIRONMENT,
};
use anyhow::{Context, Result};
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::Command;
use yolo_process::{CapturedProcessTree, ProcessIdentity};

const IDENTITY_FIELD_SEPARATOR: char = ':';
const IDENTITY_READ_CONTEXT: &str = "read historical process witness identity";
const IDENTITY_PARSE_CONTEXT: &str = "parse historical process witness identity";
const EMPTY_MARKER: &[u8] = b"";

pub(crate) struct HistoricalWitness {
    identity: OsString,
    acknowledgement: OsString,
}

pub(crate) fn capture_historical_witness() -> Option<HistoricalWitness> {
    Some(HistoricalWitness {
        identity: std::env::var_os(HISTORICAL_WITNESS_IDENTITY_ENVIRONMENT)?,
        acknowledgement: std::env::var_os(HISTORICAL_WITNESS_ACKNOWLEDGEMENT_ENVIRONMENT)?,
    })
}

impl HistoricalWitness {
    pub(crate) fn forward(self, command: &mut Command) {
        command
            .env(HISTORICAL_WITNESS_IDENTITY_ENVIRONMENT, self.identity)
            .env(
                HISTORICAL_WITNESS_ACKNOWLEDGEMENT_ENVIRONMENT,
                self.acknowledgement,
            );
    }
}

pub(crate) fn acknowledge_historical_witness(tree: &CapturedProcessTree) -> Result<()> {
    let Some(witness) = capture_historical_witness() else {
        return Ok(());
    };
    let identity_path = PathBuf::from(witness.identity);
    let Some(identity) = read_identity(&identity_path)? else {
        return Ok(());
    };
    if tree.live_identities()?.contains(&identity) {
        std::fs::write(Path::new(&witness.acknowledgement), EMPTY_MARKER)?;
    }
    Ok(())
}

fn read_identity(path: &Path) -> Result<Option<ProcessIdentity>> {
    let record = match std::fs::read_to_string(path) {
        Ok(record) => record,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error).context(IDENTITY_READ_CONTEXT),
    };
    let (pid, start_time) = record
        .split_once(IDENTITY_FIELD_SEPARATOR)
        .context(IDENTITY_PARSE_CONTEXT)?;
    Ok(Some(ProcessIdentity {
        pid: pid.parse().context(IDENTITY_PARSE_CONTEXT)?,
        start_time: start_time.parse().context(IDENTITY_PARSE_CONTEXT)?,
    }))
}
