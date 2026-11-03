pub(super) const PRIVATE_RUST_OWNERS: &[&str] = &[
    "core/crates/application/",
    "core/crates/runtime/engine/",
    "core/crates/transport/",
    "core/crates/wire/",
    "core/crates/hub/",
    "plugins/tui/local/startup/",
    "plugins/tui/client/control/",
    "core/crates/distribution/publication/protocol/",
];
pub(super) const PRIVATE_RUST_FILES: &[&str] = &[
    "core/crates/agent/providers/codex/hooks/src/framing.rs",
    "core/crates/agent/system/detection/src/process_snapshot_decode.rs",
];
pub(super) const PUBLIC_JSON_OWNERS: &[&str] = &[
    "core/crates/hub/jsonrpc/",
    "core/crates/hub/connector/jsonrpc/",
];
// Machine-readable public CLI result documents, not private IPC. Keep these
// file-scoped so a CLI component cannot use JSON for its internal traffic.
pub(super) const PUBLIC_DOCUMENT_FILES: &[&str] = &[
    "core/crates/application/upgrade/cli/src/report.rs",
    // GitHub Releases is an external public document, not Yolo-private IPC.
    "core/crates/application/upgrade/cli/src/github_release.rs",
    "core/crates/application/workspace/cli/src/commands.rs",
    "core/crates/application/workspace/cli/src/members.rs",
    "core/crates/application/workspace/cli/src/worktree.rs",
    "core/crates/application/trusted-plugin/cli/src/commands.rs",
    "core/crates/application/trusted-plugin/cli/src/execution.rs",
];
// These owners encode persisted documents or their canonical configuration digests.
// Exceptions are files, never entire Host, Worker, broker or startup components.
pub(super) const STORAGE_FILES: &[&str] = &[
    "core/crates/application/broker/profile/authority/src/document.rs",
    "core/crates/application/broker/profile/authority/src/digest.rs",
    "core/crates/runtime/engine/worker/src/startup/checkpoint/startup_checkpoint_read_io.rs",
    "core/crates/runtime/engine/host/src/reconcile/reconcile_catalog_scan.rs",
    "core/crates/runtime/engine/host/src/catalog/catalog_manifest_publication.rs",
    "core/crates/runtime/engine/host/src/creation/ledger/creation_ledger_file.rs",
    "core/crates/runtime/engine/host/src/catalog/catalog_durable_file.rs",
    "core/crates/runtime/engine/host/src/creation/receipt/creation_receipt_publish.rs",
    "core/crates/runtime/engine/host/src/resume/resume_provider_authority_record.rs",
    "core/crates/runtime/engine/host/src/creation/ledger/creation_ledger_write.rs",
    "core/crates/runtime/engine/host/src/creation/receipt/creation_receipt_read.rs",
    "core/crates/runtime/engine/host/src/resume/resume_transaction_digest.rs",
    "core/crates/runtime/engine/host/src/creation/creation_record.rs",
    "core/crates/runtime/engine/host/src/creation/ledger/creation_ledger.rs",
    "core/crates/transport/remote/src/bundle.rs",
    "core/crates/transport/remote/broker/src/config/config_snapshot.rs",
    "core/crates/transport/remote/broker/src/config/config_snapshot_material.rs",
    "core/crates/transport/remote/broker/src/config/config.rs",
    "core/crates/hub/audit/src/entry.rs",
    "core/crates/hub/audit/src/lib.rs",
    "core/crates/hub/audit/src/log.rs",
    "core/crates/runtime/engine/host/workspace/src/ledger.rs",
    "core/crates/hub/observation/store/src/size.rs",
    "core/crates/hub/ssh/entry/src/profile.rs",
    "plugins/tui/client/control/src/endpoint.rs",
];
pub(super) const JSON_BYTE_OPERATIONS: &[&str] = &[
    "to_vec",
    "to_vec_pretty",
    "to_string",
    "to_string_pretty",
    "to_writer",
    "to_writer_pretty",
    "from_slice",
    "from_str",
    "from_reader",
    "Deserializer",
    "Serializer",
];
pub(super) const PRIVATE_JSON_MESSAGE: &str = "private IPC must use Cap'n Proto; JSON byte codecs belong to explicit public or persisted-document owners";
pub(super) const SELECTION_OWNER: &str =
    "core/crates/distribution/publication/protocol/src/wire.rs";
pub(super) const SELECTION_READER: &str = "pub fn decode_selection(bytes: &[u8]) -> Result<Selection> { ensure!(bytes.len() <= MAX_PUBLICATION_BYTES, PUBLICATION_LIMIT); serde_json::from_slice(bytes).map_err(Into::into) }";
pub(super) const HUB_SERVE_OWNER: &str = "core/crates/hub/cli/src/serve.rs";
pub(super) const PUBLIC_IDENTITY_DISPLAY: &str =
    "let public_identity = serde_json::to_string(&state.peer_identity())?;";
