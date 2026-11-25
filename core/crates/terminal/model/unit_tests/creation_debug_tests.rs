use super::*;

fn assert_secret_absent<T: fmt::Debug>(value: &T, secrets: &[Uuid]) {
    let debug = format!("{value:?}");
    assert!(debug.contains(REDACTED_CREATION_AUTHORITY));
    for secret in secrets {
        assert!(!debug.contains(&secret.to_string()));
    }
}

#[test]
fn creation_model_debug_redacts_every_authority_uuid() {
    let token = Uuid::new_v4();
    let capability = Uuid::new_v4();
    let terminal_id = Uuid::new_v4();
    assert_secret_absent(
        &CreateTerminal {
            creation_token: token,
            name: None,
            cwd: PathBuf::from("/tmp"),
            cols: None,
            rows: None,
            initial_input: Some(b"secret-launch-command".to_vec()),
            context: None,
        },
        &[token],
    );
    let create = CreateTerminal {
        creation_token: token,
        name: None,
        cwd: PathBuf::from("/tmp"),
        cols: None,
        rows: None,
        initial_input: Some(b"secret-launch-command".to_vec()),
        context: None,
    };
    assert!(!format!("{create:?}").contains("secret-launch-command"));
    assert_secret_absent(
        &CreateTerminalResult {
            terminal_id,
            creation_capability: capability,
        },
        &[capability],
    );
    assert_secret_absent(
        &FinalizeCreationRequest {
            creation_token: token,
            terminal_id,
            creation_capability: capability,
        },
        &[token, capability],
    );
    assert_secret_absent(
        &CreationCleanupArchiveRequest {
            terminal_id,
            creation_capability: capability,
        },
        &[capability],
    );
    assert_secret_absent(
        &CreationCleanupTerminateRequest {
            terminal_id,
            creation_capability: capability,
            force: true,
        },
        &[capability],
    );
    assert_secret_absent(
        &CreationCleanupDiscardRequest {
            terminal_id,
            creation_capability: capability,
            force: true,
        },
        &[capability],
    );
}
