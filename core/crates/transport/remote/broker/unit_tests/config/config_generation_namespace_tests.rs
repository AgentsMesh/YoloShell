use super::tests::Fixture;
use super::*;
use crate::remote_broker::BROKER_READY_TIMEOUT;
use crate::remote_broker::constants::BROKER_NAMESPACE_LOST_CONTEXT;
use std::fs;
use std::os::unix::fs::PermissionsExt;

const DISPLACED_STATE_DIRECTORY: &str = "generation-displaced-state";
const SENTINEL_FILE: &str = "replacement-sentinel";
const SENTINEL_BYTES: &[u8] = b"preserve-replacement";
const DISPLACED_GENERATION_ROOT: &str = "committed-generation-root";
const GENERATION_ROOT_CHANGED_MESSAGE: &str = "private directory path changed";
const INJECTED_POST_RENAME_FAILURE: &str = "injected post-rename failure";

#[tokio::test]
async fn guarded_stage_never_publishes_into_replacement_state() {
    let fixture = Fixture::new();
    let guard = BrokerReconcileGuard::acquire(
        &fixture.paths,
        tokio::time::Instant::now() + BROKER_READY_TIMEOUT,
    )
    .await
    .unwrap();
    let state = fixture.paths.state_dir().to_owned();
    let displaced = state.parent().unwrap().join(DISPLACED_STATE_DIRECTORY);
    let mut sentinel = None;

    let error =
        stage_broker_config_guarded_with_hook(&fixture.source, &fixture.paths, &guard, |stage| {
            if stage == GenerationPublishStage::SourceVerified {
                fs::rename(&state, &displaced)?;
                let replacement = BrokerPaths::new(&state)?;
                let path = replacement.state_dir().join(SENTINEL_FILE);
                fs::write(&path, SENTINEL_BYTES)?;
                sentinel = Some(path);
            }
            Ok(())
        })
        .unwrap_err();

    assert!(error.to_string().contains(BROKER_NAMESPACE_LOST_CONTEXT));
    assert!(guard.verify().is_err());
    assert!(guard.verify().is_err());
    assert_eq!(fs::read(sentinel.unwrap()).unwrap(), SENTINEL_BYTES);
    assert!(!state.join(CONFIG_GENERATIONS_DIRECTORY).exists());
    assert!(displaced.join(CONFIG_GENERATIONS_DIRECTORY).is_dir());
}

#[test]
fn post_rename_verification_failure_preserves_sealed_generation() {
    let fixture = Fixture::new();
    let root = fixture.paths.generations();
    let displaced = fixture.paths.state_dir().join(DISPLACED_GENERATION_ROOT);
    let sentinel = root.join(SENTINEL_FILE);
    let error = stage_broker_config_with_hook(&fixture.source, &fixture.paths, |stage| {
        if stage == GenerationPublishStage::DestinationRenamed {
            fs::rename(&root, &displaced)?;
            fs::create_dir(&root)?;
            fs::write(&sentinel, SENTINEL_BYTES)?;
            fs::set_permissions(&root, fs::Permissions::from_mode(PRIVATE_DIRECTORY_MODE))?;
        }
        Ok(())
    })
    .unwrap_err();

    assert!(error.to_string().contains(GENERATION_ROOT_CHANGED_MESSAGE));
    assert_eq!(fs::read(&sentinel).unwrap(), SENTINEL_BYTES);
    let committed = fs::read_dir(&displaced)
        .unwrap()
        .filter_map(Result::ok)
        .find(|entry| {
            !entry
                .file_name()
                .to_string_lossy()
                .starts_with(PENDING_GENERATION_PREFIX)
        })
        .unwrap()
        .path();
    let mode = fs::metadata(&committed).unwrap().permissions().mode() & PERMISSION_BITS_MASK;
    assert_eq!(mode, IMMUTABLE_DIRECTORY_MODE);
    assert!(committed.join(GENERATION_CONFIG_FILE).is_file());
}

#[test]
fn retry_after_post_rename_failure_reconfirms_root_durability() {
    let fixture = Fixture::new();
    let first = stage_broker_config_with_hook(&fixture.source, &fixture.paths, |stage| {
        if stage == GenerationPublishStage::DestinationRenamed {
            anyhow::bail!(INJECTED_POST_RENAME_FAILURE);
        }
        Ok(())
    })
    .unwrap_err();
    assert!(first.to_string().contains(INJECTED_POST_RENAME_FAILURE));

    let mut confirmed = false;
    let generation = stage_broker_config_with_hook(&fixture.source, &fixture.paths, |stage| {
        confirmed |= stage == GenerationPublishStage::ExistingDurabilityConfirmed;
        Ok(())
    })
    .unwrap();

    assert!(confirmed);
    assert!(generation.config_path().is_file());
}
