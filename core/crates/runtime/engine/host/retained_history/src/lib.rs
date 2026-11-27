//! Read the retained output of an archived terminal through the Host-owned
//! namespace. The Worker is gone, so this opens the durable journal through its
//! public recovery API and returns one bounded raw stream for client replay.

use base64::Engine;
use yolo_journal::{Journal, read_snapshot};
use yolo_paths::namespace_lease::NamespaceLease;
use yolo_paths::Paths;
use yolo_runtime_protocol::{RetainedHistoryRequest, RetainedHistoryResult};
use yolo_terminal_model::{RuntimeRecord, TerminalManifest};
use yolo_wire::ErrorBody;

const MAX_BYTES: u32 = 4 * 1024 * 1024;
const UNAVAILABLE: &str = "retained history is unavailable";
const NOT_ARCHIVED: &str = "retained history is only available for archived sessions";
const RUNTIME_MISMATCH: &str = "retained history runtime epoch does not match";

pub fn read(
    namespace: &NamespaceLease,
    paths: &Paths,
    request: RetainedHistoryRequest,
) -> Result<RetainedHistoryResult, ErrorBody> {
    if request.max_bytes == 0 || request.max_bytes > MAX_BYTES {
        return Err(ErrorBody::invalid("retained history byte budget is invalid"));
    }
    let terminal_id = request.terminal_id;
    let manifest: TerminalManifest = namespace
        .read_json(&paths.manifest(terminal_id))
        .map_err(|_| ErrorBody::not_found(UNAVAILABLE))?;
    if !manifest.archived || manifest.terminal_id != terminal_id {
        return Err(ErrorBody::conflict(NOT_ARCHIVED));
    }
    let runtime: RuntimeRecord = namespace
        .read_json(&paths.runtime_record(terminal_id))
        .map_err(|_| ErrorBody::not_found(UNAVAILABLE))?;
    if runtime.terminal_id != terminal_id || runtime.runtime_epoch != request.runtime_epoch {
        return Err(ErrorBody::conflict(RUNTIME_MISMATCH));
    }
    let (data, from_sequence, next_sequence, truncated) = read_stream(namespace, paths, &request)?;
    Ok(RetainedHistoryResult {
        runtime_epoch: request.runtime_epoch,
        data_base64: base64::engine::general_purpose::STANDARD.encode(data),
        from_sequence,
        next_sequence,
        truncated,
    })
}

fn read_stream(
    namespace: &NamespaceLease,
    paths: &Paths,
    request: &RetainedHistoryRequest,
) -> Result<(Vec<u8>, u64, u64, bool), ErrorBody> {
    let mut journal = open_journal(namespace, paths, request)?;
    let retained_from = journal.retained_from_sequence();
    let next_sequence = journal.last_sequence().saturating_add(1);
    if retained_from > 1 {
        if let Some(snapshot) = snapshot(namespace, paths, request) {
            return Ok((
                bounded(snapshot.data, request.max_bytes),
                snapshot.barrier_sequence,
                next_sequence,
                true,
            ));
        }
    }
    let chunks = journal
        .replay_after(0, request.max_bytes as usize)
        .map_err(|_| ErrorBody::internal(UNAVAILABLE))?;
    let from_sequence = chunks.first().map_or(1, |chunk| chunk.sequence);
    let last_sequence = chunks.last().map_or(0, |chunk| chunk.sequence);
    let truncated = retained_from > 1 || last_sequence < journal.last_sequence();
    let mut data = Vec::new();
    for chunk in chunks {
        data.extend_from_slice(&chunk.data);
    }
    Ok((data, from_sequence, next_sequence, truncated))
}

fn open_journal(
    namespace: &NamespaceLease,
    paths: &Paths,
    request: &RetainedHistoryRequest,
) -> Result<Journal, ErrorBody> {
    let path = paths.journal(request.terminal_id);
    if namespace
        .file_is_missing(&path)
        .map_err(|_| ErrorBody::internal(UNAVAILABLE))?
    {
        return Err(ErrorBody::not_found(UNAVAILABLE));
    }
    let entry = namespace
        .file(&path)
        .map_err(|_| ErrorBody::not_found(UNAVAILABLE))?;
    Journal::open_retained(entry, request.runtime_epoch)
        .map_err(|_| ErrorBody::internal(UNAVAILABLE))
}

fn snapshot(
    namespace: &NamespaceLease,
    paths: &Paths,
    request: &RetainedHistoryRequest,
) -> Option<yolo_terminal_model::TerminalSnapshot> {
    let path = paths.snapshot(request.terminal_id);
    if namespace.file_is_missing(&path).ok()? {
        return None;
    }
    let snapshot = read_snapshot(&path).ok()?;
    (snapshot.runtime_epoch == request.runtime_epoch).then_some(snapshot)
}

fn bounded(mut data: Vec<u8>, max_bytes: u32) -> Vec<u8> {
    let limit = max_bytes as usize;
    if data.len() <= limit {
        return data;
    }
    let excess = data.len() - limit;
    data.drain(..excess);
    data
}
