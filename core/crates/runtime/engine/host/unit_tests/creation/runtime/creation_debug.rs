use super::*;
use crate::host::tests::*;

#[test]
fn creation_ledger_debug_redacts_capability_and_launch_authentication() {
    let capability = Uuid::new_v4();
    let auth_token = format!("{}{}", Uuid::new_v4().simple(), Uuid::new_v4().simple());
    let binding = CreationBinding {
        creation_token_digest: yolo_crypto::sha256_hex(Uuid::new_v4().as_bytes()),
        result: CreateTerminalResult {
            terminal_id: Uuid::new_v4(),
            creation_capability: capability,
        },
        spec: CreationSpec {
            request_fingerprint: yolo_crypto::sha256_hex(b"debug-fixture"),
            name: None,
            cwd: PathBuf::from("/tmp"),
            cols: None,
            rows: None,
            initial_input: Some(b"secret-launch-command".to_vec()),
            context: None,
        },
        phase: CreationPhase::Launching,
        launch_generation: INITIAL_CREATION_LAUNCH_GENERATION,
        launch: Some(CreationLaunchIdentity {
            runtime_epoch: Uuid::new_v4(),
            worker_id: Uuid::new_v4(),
            auth_token: auth_token.clone(),
        }),
    };
    let ledger = CreationLedger {
        bindings: vec![binding],
        ..CreationLedger::default()
    };

    let debug = format!("{ledger:?}");
    assert!(debug.contains(REDACTED_CREATION_RECORD_SECRET));
    assert!(!debug.contains(&capability.to_string()));
    assert!(!debug.contains(&auth_token));
    assert!(!debug.contains("secret-launch-command"));
}
