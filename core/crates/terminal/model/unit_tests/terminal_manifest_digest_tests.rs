use super::terminal_manifest_digest_test_constants::*;
use std::path::PathBuf;
use uuid::Uuid;
use yolo_terminal_model::{RecoveryState, TerminalManifest, TerminalManifestDigest};

fn manifest() -> TerminalManifest {
    TerminalManifest {
        schema_version: SCHEMA_VERSION,
        terminal_id: Uuid::parse_str(TERMINAL_ID).unwrap(),
        name: NAME.into(),
        command: COMMAND.into(),
        args: vec![LOGIN_ARGUMENT.into(), FLAG_ARGUMENT.into()],
        cwd: PathBuf::from(WORKING_DIRECTORY),
        cols: COLUMNS,
        rows: ROWS,
        created_at_ms: CREATED_AT_MS,
        updated_at_ms: UPDATED_AT_MS,
        archived: true,
        recovery_state: RecoveryState::NeedsRecreate,
    }
}

#[test]
fn digest_covers_every_manifest_field_with_a_stable_vector() {
    let original = manifest();
    let digest = TerminalManifestDigest::capture(&original).unwrap();
    assert_eq!(digest.as_str(), EXPECTED_DIGEST);

    let mutations = vec![
        TerminalManifest {
            schema_version: original.schema_version + U16_MUTATION_INCREMENT,
            ..original.clone()
        },
        TerminalManifest {
            terminal_id: Uuid::parse_str(REPLACEMENT_TERMINAL_ID).unwrap(),
            ..original.clone()
        },
        TerminalManifest {
            name: REPLACEMENT_NAME.into(),
            ..original.clone()
        },
        TerminalManifest {
            command: REPLACEMENT_COMMAND.into(),
            ..original.clone()
        },
        TerminalManifest {
            args: vec![LOGIN_ARGUMENT.into()],
            ..original.clone()
        },
        TerminalManifest {
            cwd: PathBuf::from(REPLACEMENT_WORKING_DIRECTORY),
            ..original.clone()
        },
        TerminalManifest {
            cols: original.cols + U16_MUTATION_INCREMENT,
            ..original.clone()
        },
        TerminalManifest {
            rows: original.rows + U16_MUTATION_INCREMENT,
            ..original.clone()
        },
        TerminalManifest {
            created_at_ms: original.created_at_ms + I64_MUTATION_INCREMENT,
            ..original.clone()
        },
        TerminalManifest {
            updated_at_ms: original.updated_at_ms + I64_MUTATION_INCREMENT,
            ..original.clone()
        },
        TerminalManifest {
            archived: !original.archived,
            ..original.clone()
        },
        TerminalManifest {
            recovery_state: RecoveryState::Uncertain,
            ..original.clone()
        },
    ];
    for changed in mutations {
        assert_ne!(TerminalManifestDigest::capture(&changed).unwrap(), digest);
    }
}

#[test]
fn digest_wire_rejects_noncanonical_sha256() {
    let digest = TerminalManifestDigest::capture(&manifest()).unwrap();
    let encoded = serde_json::to_string(&digest).unwrap();
    assert_eq!(
        serde_json::from_str::<TerminalManifestDigest>(&encoded).unwrap(),
        digest
    );
    assert!(serde_json::from_str::<TerminalManifestDigest>(NONCANONICAL_DIGEST_JSON).is_err());
}
