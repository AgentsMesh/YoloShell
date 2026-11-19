use serde_json::{Value, json};
use yolo_release_evidence_model::{BazelLauncherEvidence, GateToolEvidence};

const UNKNOWN_FIELD_DOCUMENT: &str = r#"{
  "canonical_path": "/private/bazel",
  "execution_path": "/private/bazel",
  "sha256": "digest",
  "version": "bazel 9",
  "unexpected": true
}"#;

#[test]
fn evidence_records_reject_unknown_fields() {
    assert!(serde_json::from_str::<BazelLauncherEvidence>(UNKNOWN_FIELD_DOCUMENT).is_err());
}

#[test]
fn gate_tool_kind_is_an_explicit_stable_tag() {
    let evidence = GateToolEvidence::MacOsBazel {
        launcher: BazelLauncherEvidence {
            canonical_path: "/private/bazel".to_owned(),
            execution_path: "/private/bazel".to_owned(),
            sha256: "digest".to_owned(),
            version: "bazel 9".to_owned(),
        },
    };
    let encoded = serde_json::to_value(&evidence).unwrap();
    assert_eq!(encoded["kind"], json!("mac_os_bazel"));
    let decoded: GateToolEvidence = serde_json::from_value::<GateToolEvidence>(encoded).unwrap();
    assert_eq!(decoded, evidence);
}

#[test]
fn malformed_or_unknown_gate_kinds_are_rejected() {
    let unknown: Value = json!({"kind": "shell", "command": "bazel"});
    assert!(serde_json::from_value::<GateToolEvidence>(unknown).is_err());
    assert!(serde_json::from_slice::<GateToolEvidence>(b"not-json").is_err());
}
