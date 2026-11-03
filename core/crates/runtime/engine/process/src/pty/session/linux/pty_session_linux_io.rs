use super::super::constants::LINUX_PTY_PROOF_STATUS_BYTES;
use super::super::procfs;
use super::linux_contract::{BudgetKind, CandidatePending, QueryKind, WorkWindow};
use std::io::{self, Read};

const EFFECTIVE_UID_INDEX: usize = 1;
const FIELD_SEPARATOR: u8 = 0;
const READ_SENTINEL_BYTES: usize = 1;
const STATUS_UID_FIELD: &str = "Uid:";

pub(super) enum QueryValue<T> {
    Value(T),
    Pending(CandidatePending),
}

pub(super) struct EnvironmentQuery {
    pub(super) value: QueryValue<bool>,
    pub(super) bytes: usize,
}

pub(super) fn effective_uid(pid: u32) -> QueryValue<u32> {
    match read_bounded(procfs::status_path(pid), LINUX_PTY_PROOF_STATUS_BYTES) {
        Ok(ReadResult::Complete(bytes)) => parse_effective_uid(&bytes)
            .map(QueryValue::Value)
            .unwrap_or(QueryValue::Pending(CandidatePending::Query(
                QueryKind::Status,
            ))),
        Ok(ReadResult::Exceeded(_)) => {
            QueryValue::Pending(CandidatePending::Budget(BudgetKind::CandidateStatus))
        }
        Err(error) => QueryValue::Pending(io_pending(QueryKind::Status, &error)),
    }
}

pub(super) fn environment_capability(
    pid: u32,
    environment_name: &str,
    capability: &str,
    window: WorkWindow,
) -> EnvironmentQuery {
    if window.limit == usize::default() {
        return EnvironmentQuery {
            value: QueryValue::Pending(CandidatePending::Budget(window.exhausted)),
            bytes: usize::default(),
        };
    }
    match read_bounded(procfs::environment_path(pid), window.limit) {
        Ok(ReadResult::Complete(bytes)) => EnvironmentQuery {
            value: parse_capability(&bytes, environment_name, capability),
            bytes: bytes.len(),
        },
        Ok(ReadResult::Exceeded(bytes)) => EnvironmentQuery {
            value: QueryValue::Pending(CandidatePending::Budget(window.exhausted)),
            bytes,
        },
        Err(error) => EnvironmentQuery {
            value: QueryValue::Pending(io_pending(QueryKind::Environment, &error)),
            bytes: usize::default(),
        },
    }
}

pub(super) fn io_pending(kind: QueryKind, error: &io::Error) -> CandidatePending {
    match error.raw_os_error() {
        Some(libc::EACCES | libc::EPERM) => CandidatePending::Permission(kind),
        _ => CandidatePending::Query(kind),
    }
}

fn parse_capability(
    environment: &[u8],
    environment_name: &str,
    capability: &str,
) -> QueryValue<bool> {
    if environment
        .last()
        .is_some_and(|byte| *byte != FIELD_SEPARATOR)
    {
        return QueryValue::Pending(CandidatePending::Query(QueryKind::Environment));
    }
    let expected = format!("{environment_name}={capability}");
    QueryValue::Value(
        environment
            .split(|byte| *byte == FIELD_SEPARATOR)
            .any(|field| field == expected.as_bytes()),
    )
}

fn parse_effective_uid(status: &[u8]) -> Option<u32> {
    let status = std::str::from_utf8(status).ok()?;
    let values = status
        .lines()
        .find_map(|line| line.strip_prefix(STATUS_UID_FIELD))?;
    values
        .split_whitespace()
        .nth(EFFECTIVE_UID_INDEX)?
        .parse()
        .ok()
}

enum ReadResult {
    Complete(Vec<u8>),
    Exceeded(usize),
}

fn read_bounded(path: std::path::PathBuf, limit: usize) -> io::Result<ReadResult> {
    let capacity = limit.saturating_add(READ_SENTINEL_BYTES);
    let mut bytes = Vec::with_capacity(capacity.min(LINUX_PTY_PROOF_STATUS_BYTES));
    std::fs::File::open(path)?
        .take(u64::try_from(capacity).unwrap_or(u64::MAX))
        .read_to_end(&mut bytes)?;
    if bytes.len() > limit {
        return Ok(ReadResult::Exceeded(bytes.len()));
    }
    Ok(ReadResult::Complete(bytes))
}

#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-integration-tests"))]
#[path = "../../../../integration_tests/pty/session/linux/pty_session_linux_io_tests.rs"]
mod tests;
