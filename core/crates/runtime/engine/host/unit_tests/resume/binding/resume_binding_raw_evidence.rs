use crate::host::tests::*;
use super::*;

const RAW_BINDING_CORRUPT: &[u8] = b"{invalid-agent-binding";
const RAW_BINDING_TRUNCATED: &[u8] = b"{\"schema_version\":3";
const RAW_BINDING_SAVED_EXTENSION: &str = "saved";

#[test]
fn resume_binding_evidence_rejects_parsed_binding_records() {
    let binding = host_binding(Uuid::new_v4());
    let encoded = serde_json::to_value(binding).unwrap();
    assert!(serde_json::from_value::<ResumeBindingEvidence>(encoded).is_err());
}

#[test]
fn resume_binding_clear_requires_the_captured_file_even_with_identical_json() {
    let directory = tempdir().unwrap();
    let host = Host::open(HostConfig::new(directory.path())).unwrap();
    let terminal_id = Uuid::new_v4();
    host.paths.ensure_session(terminal_id).unwrap();
    let binding = host_binding(terminal_id);
    let path = host.paths.agent_binding(terminal_id);
    let store = AgentBindingStore::new(&path);
    store.save(&binding).unwrap();
    let evidence = host
        .capture_resume_binding_evidence(&path)
        .unwrap()
        .unwrap();
    let encoded = serde_json::to_value(evidence).unwrap();
    let restored: ResumeBindingEvidence = serde_json::from_value(encoded).unwrap();
    assert_eq!(restored, evidence);
    std::fs::rename(&path, path.with_extension(RAW_BINDING_SAVED_EXTENSION)).unwrap();
    store.save(&binding).unwrap();
    assert!(
        host.clear_resume_binding_evidence(&path, &restored)
            .is_err()
    );
    assert_eq!(read_json::<AgentBinding>(&path).unwrap(), binding);
}

#[tokio::test]
async fn explicit_recreate_clears_legacy_and_malformed_binding_bytes() {
    let directory = tempdir().unwrap();
    let host = Host::open(HostConfig::new(directory.path())).unwrap();
    for (label, payload) in invalid_binding_payloads(host_binding(Uuid::new_v4())) {
        let mut manifest = fixture_manifest(directory.path());
        host.paths.ensure_session(manifest.terminal_id).unwrap();
        let path = host.paths.agent_binding(manifest.terminal_id);
        write_private_bytes(&path, &payload);
        let transaction = capture_raw_clear_transaction(&host, &mut manifest).await;

        host.stage_resume_binding_disposition(&transaction)
            .unwrap_or_else(|error| panic!("clear {label}: {error}"));
        assert!(!path.exists(), "binding survived clear: {label}");
        host.stage_resume_binding_disposition(&transaction)
            .unwrap_or_else(|error| panic!("replay {label}: {error}"));
        host.apply_resume_binding_disposition(&transaction).unwrap();
        host.complete_resume_binding_disposition(&transaction)
            .unwrap();
        assert!(
            !host
                .paths
                .resume_binding_clear(manifest.terminal_id)
                .exists()
        );
    }
}

#[tokio::test]
async fn raw_binding_clear_rejects_replacement_on_every_retry() {
    let directory = tempdir().unwrap();
    let host = Host::open(HostConfig::new(directory.path())).unwrap();
    let mut manifest = fixture_manifest(directory.path());
    host.paths.ensure_session(manifest.terminal_id).unwrap();
    let path = host.paths.agent_binding(manifest.terminal_id);
    write_private_bytes(&path, RAW_BINDING_CORRUPT);
    let transaction = capture_raw_clear_transaction(&host, &mut manifest).await;
    let replacement = host_binding(manifest.terminal_id);
    AgentBindingStore::new(&path).save(&replacement).unwrap();

    assert!(host.stage_resume_binding_disposition(&transaction).is_err());
    assert!(host.stage_resume_binding_disposition(&transaction).is_err());
    assert_eq!(read_json::<AgentBinding>(&path).unwrap(), replacement);
    let marker: ResumeBindingClearMarker =
        read_json(&host.paths.resume_binding_clear(manifest.terminal_id)).unwrap();
    assert_eq!(marker.phase, ResumeBindingClearPhase::Pending);
}

async fn capture_raw_clear_transaction(
    host: &Host,
    manifest: &mut TerminalManifest,
) -> ResumeTransaction {
    let selected = host
        .binding_for_resume(manifest, ResumeSelection::ExplicitRecreate)
        .await
        .unwrap();
    assert!(matches!(
        &selected,
        ResumeBinding::Shell(ResumeBindingDisposition::Clear {
            previous: Some(ResumeBindingEvidence { .. }),
            ..
        })
    ));
    let launch = shell_resume_launch(host, manifest);
    ResumeTransaction::new(
        Uuid::new_v4(),
        ResumeIntent::Explicit,
        ResumeTarget::from_binding(&selected),
        &launch,
    )
    .unwrap()
}

fn invalid_binding_payloads(binding: AgentBinding) -> Vec<(&'static str, Vec<u8>)> {
    let current = serde_json::to_value(binding).unwrap();
    let mut legacy = current.clone();
    legacy[crate::agent::constants::SCHEMA_VERSION_FIELD] =
        serde_json::json!(crate::agent::constants::BINDING_SCHEMA_VERSION.saturating_sub(1));
    legacy
        .as_object_mut()
        .unwrap()
        .remove(crate::agent::constants::RECOVERY_AVAILABILITY_FIELD);
    let mut missing = current.clone();
    missing
        .as_object_mut()
        .unwrap()
        .remove(crate::agent::constants::DATA_HOME_FIELD);
    let mut unknown = current;
    unknown.as_object_mut().unwrap().insert(
        crate::protocol::field::COMMAND.into(),
        serde_json::Value::Bool(true),
    );
    vec![
        ("legacy v2", serde_json::to_vec(&legacy).unwrap()),
        ("missing field", serde_json::to_vec(&missing).unwrap()),
        ("unknown field", serde_json::to_vec(&unknown).unwrap()),
        ("corrupt JSON", RAW_BINDING_CORRUPT.to_vec()),
        ("truncated JSON", RAW_BINDING_TRUNCATED.to_vec()),
    ]
}
