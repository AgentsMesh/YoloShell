use crate::{AuditLog, AuditOutcome, AuditTrust, MAX_AUDIT_RECORDS, read};
use std::sync::Arc;
use uuid::Uuid;
use yolo_agent_input::Prompt;
use yolo_hub_agent_protocol::{
    AGENT_PROTOCOL_SCHEMA_VERSION, AgentPromptFields, AgentPromptReference, AgentPromptRequest,
    AgentPromptWait,
};
use yolo_hub_delegation_test_support::Identity;
use yolo_hub_identity::{TerminalAddress, TerminalId};
use yolo_hub_rpc_protocol::Command;
use yolo_hub_terminal_protocol::{
    CreateShellResult, SecretCapability, TerminalAction, TerminalCommand, TerminalSelection,
};

mod identity;

#[test]
fn records_are_signed_durable_and_cursor_readable() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join(crate::AUDIT_FILE);
    let signer = Identity::new();
    let hub = signer.identity.hub();
    let log = AuditLog::open(path.clone(), Arc::new(signer)).unwrap();
    log.record("local", hub, &Command::Hello {}, AuditOutcome::Accepted)
        .unwrap();
    log.record("local", hub, &Command::Hello {}, AuditOutcome::Failed)
        .unwrap();
    let entries = read(&path).unwrap();
    assert_eq!(entries.len(), 2);
    assert_eq!(entries[0].record.sequence, 1);
    assert_eq!(entries[0].record.outcome, AuditOutcome::Accepted);
    assert_eq!(entries[1].record.outcome, AuditOutcome::Failed);
    assert_eq!(entries[1].record.destination, hub);
    assert!(entries.iter().all(|entry| entry.signature_valid));
    assert!(entries.iter().all(|entry| entry.authenticated()));
    assert_eq!(MAX_AUDIT_RECORDS, 4096);
    assert!(log.records().len() == 2);
}

fn terminal(hub: yolo_hub_identity::HubId) -> (TerminalAddress, Uuid) {
    let request_id = Uuid::new_v4();
    (
        TerminalAddress {
            hub_id: hub,
            terminal_id: TerminalId::new(),
        },
        request_id,
    )
}

#[test]
fn terminal_and_creation_envelopes_keep_audit_identity() {
    let hub = yolo_hub_identity::HubId::new();
    let (address, request_id) = terminal(hub);
    let command = Command::Terminal {
        command: TerminalCommand {
            request_id,
            selection: TerminalSelection {
                address,
                expected_runtime: None,
            },
            action: TerminalAction::Terminate { force: true },
        },
    };
    let id = address.terminal_id.to_string();
    assert_eq!(
        crate::extract::target(&command).as_deref(),
        Some(id.as_str())
    );
    assert_eq!(crate::extract::request_id(&command), Some(request_id));

    let creation = CreateShellResult {
        address,
        creation_capability: SecretCapability::generate(),
    };
    let cleanup = Command::CleanupTerminate {
        creation,
        force: false,
    };
    assert_eq!(
        crate::extract::target(&cleanup).as_deref(),
        Some(id.as_str())
    );

    let prompt = AgentPromptRequest::new(AgentPromptFields {
        schema_version: AGENT_PROTOCOL_SCHEMA_VERSION,
        request_id,
        reference: AgentPromptReference::Terminal { address },
        prompt: Prompt::new("inspect".into()).unwrap(),
        wait: AgentPromptWait::None,
        take_control: false,
    })
    .unwrap();
    let prompt = Command::AgentPrompt { request: prompt };
    assert_eq!(
        crate::extract::target(&prompt).as_deref(),
        Some(id.as_str())
    );
    assert_eq!(crate::extract::request_id(&prompt), Some(request_id));
}

#[test]
fn tampering_with_a_signed_record_invalidates_its_signature() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join(crate::AUDIT_FILE);
    let signer = Identity::new();
    let hub = signer.identity.hub();
    let log = AuditLog::open(path.clone(), Arc::new(signer)).unwrap();
    log.record("local", hub, &Command::Hello {}, AuditOutcome::Accepted)
        .unwrap();
    let text = std::fs::read_to_string(&path).unwrap();
    let mut value: serde_json::Value = serde_json::from_str(text.trim()).unwrap();
    value["payload"]["outcome"] = serde_json::json!("denied");
    std::fs::write(&path, format!("{value}\n")).unwrap();
    let entries = read(&path).unwrap();
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].record.outcome, AuditOutcome::Denied);
    assert!(!entries[0].signature_valid);
    assert!(!entries[0].authenticated());
}

#[test]
fn trust_requires_the_expected_signer_for_the_destination() {
    let signer = Identity::new();
    let other = Identity::new();
    let hub = signer.identity.hub();
    let trusted = AuditTrust::parse(&[format!(
        "{}={}",
        hub,
        crate::public_key_hex(&signer.identity)
    )])
    .unwrap();
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join(crate::AUDIT_FILE);
    let log = AuditLog::open(path.clone(), Arc::new(signer)).unwrap();
    log.record("local", hub, &Command::Hello {}, AuditOutcome::Accepted)
        .unwrap();
    let entry = read(&path).unwrap().remove(0);
    assert!(trusted.accepts(&entry));

    let foreign = AuditTrust::parse(&[format!(
        "{}={}",
        other.identity.hub(),
        crate::public_key_hex(&other.identity)
    )])
    .unwrap();
    assert!(!foreign.accepts(&entry));
    assert!(AuditTrust::parse(&["not-a-trust".into()]).is_err());
}

#[test]
fn export_reads_bounded_signed_lines_after_a_cursor() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join(crate::AUDIT_FILE);
    let signer = Identity::new();
    let hub = signer.identity.hub();
    let log = AuditLog::open(path.clone(), Arc::new(signer)).unwrap();
    for _ in 0..3 {
        log.record("local", hub, &Command::Hello {}, AuditOutcome::Accepted)
            .unwrap();
    }
    let lines = crate::read_export(&path, Some(1), 2).unwrap();
    assert_eq!(lines.len(), 2);
    assert_eq!(lines[0]["payload"]["sequence"], 2);
    assert_eq!(lines[1]["payload"]["sequence"], 3);
    assert!(lines.iter().all(|line| line.get("signature").is_some()));
}
