use super::managed_mutation_support::ManagedFixture;
use crate::agent::AgentState;
use crate::agent::claude::generation::adversarial_test_support::{
    PersistenceBoundary, persist_durable_state_with_boundary,
};
use crate::paths::durable_mutation::DurableMutationFencePhase;
use std::io::Write;
use std::os::unix::fs::OpenOptionsExt;
use std::rc::Rc;

const ARCHIVED: bool = true;
const NOT_ARCHIVED: bool = false;

#[test]
fn managed_generation_creation_succeeds_with_current_authority() {
    let fixture = ManagedFixture::new(NOT_ARCHIVED, None);

    let authority = fixture.begin().unwrap();

    assert_eq!(authority.terminal_id, fixture.session_id);
    assert!(fixture.store.path().exists());
}

#[test]
fn pending_intent_blocks_absent_generation_creation() {
    let fixture = ManagedFixture::new(NOT_ARCHIVED, None);
    publish_intent(&fixture.intent()).unwrap();

    assert!(fixture.begin().is_err());
    assert!(!fixture.store.path().exists());
}

#[test]
fn intent_arriving_at_commit_boundary_blocks_creation() {
    let fixture = ManagedFixture::new(NOT_ARCHIVED, None);
    let intent = fixture.intent();
    install_once(DurableMutationFencePhase::Commit, move || {
        publish_intent(&intent)
    });

    assert!(fixture.begin().is_err());
    assert!(!fixture.store.path().exists());
}

#[test]
fn archived_and_foreign_manifests_fail_closed() {
    for fixture in [
        ManagedFixture::new(ARCHIVED, None),
        ManagedFixture::new(NOT_ARCHIVED, Some(uuid::Uuid::new_v4())),
    ] {
        assert!(fixture.begin().is_err());
        assert!(!fixture.store.path().exists());
    }
}

#[test]
fn replaced_session_directory_rejects_creation() {
    let fixture = ManagedFixture::new(NOT_ARCHIVED, None);
    let session = fixture.paths.session_dir(fixture.session_id);
    let displaced = fixture
        .paths
        .sessions_dir()
        .join(uuid::Uuid::new_v4().to_string());
    install_once(DurableMutationFencePhase::Commit, move || {
        std::fs::rename(&session, &displaced)?;
        crate::paths::directory::PrivateDirectory::ensure(&session)?;
        Ok(())
    });

    assert!(fixture.begin().is_err());
    assert!(!fixture.store.path().exists());
}

#[test]
fn exact_generation_identity_rejects_a_stale_writer() {
    let fixture = ManagedFixture::new(NOT_ARCHIVED, None);
    fixture.begin().unwrap();
    let (_, original_identity) = fixture
        .lease
        .read_json_with_identity::<super::super::ClaudeInvocationAuthorities>(fixture.store.path())
        .unwrap();
    let path = fixture.store.path().to_owned();
    install_once(DurableMutationFencePhase::Commit, move || {
        replace_generation(&path)
    });
    let mut begin = fixture.begin.clone();
    begin.runtime_epoch = uuid::Uuid::new_v4();
    begin.invocation_id = uuid::Uuid::new_v4();
    begin.hook_token = uuid::Uuid::new_v4();

    assert!(
        fixture
            .store
            .begin_in_namespace(&fixture.lease, &begin)
            .is_err()
    );
    let (_, replacement_identity) = fixture
        .lease
        .read_json_with_identity::<super::super::ClaudeInvocationAuthorities>(fixture.store.path())
        .unwrap();
    assert_ne!(replacement_identity, original_identity);
}

#[test]
fn durable_binding_transaction_holds_one_lifecycle_capability() {
    let fixture = ManagedFixture::new(NOT_ARCHIVED, None);
    fixture.publish();
    let paths = fixture.paths.clone();
    let session_id = fixture.session_id;
    let mut boundaries = Vec::new();

    persist_durable_state_with_boundary(
        &fixture.paths.agent_binding(session_id),
        AgentState::WaitingForInput,
        |boundary| {
            assert!(
                paths
                    .try_acquire_session_lifecycle_lock(session_id)?
                    .is_none()
            );
            boundaries.push(boundary);
            Ok(())
        },
    )
    .unwrap();

    assert_eq!(
        boundaries,
        [
            PersistenceBoundary::AuthorityPending,
            PersistenceBoundary::BindingWritten,
        ]
    );
}

fn install_once(
    phase: DurableMutationFencePhase,
    action: impl FnOnce() -> anyhow::Result<()> + 'static,
) {
    let action = Rc::new(std::cell::RefCell::new(Some(action)));
    crate::agent::claude::generation::mutation::set_test_hook(move |actual| {
        if actual == phase {
            if let Some(action) = action.borrow_mut().take() {
                return action();
            }
        }
        Ok(())
    });
}

fn publish_intent(path: &std::path::Path) -> anyhow::Result<()> {
    crate::paths::atomic_write_json(path, &serde_json::Value::Null)
}

fn replace_generation(path: &std::path::Path) -> anyhow::Result<()> {
    let value: super::super::ClaudeInvocationAuthorities = crate::paths::read_json(path)?;
    let encoded = serde_json::to_vec_pretty(&value)?;
    std::fs::remove_file(path)?;
    let mut replacement = std::fs::OpenOptions::new()
        .create_new(true)
        .write(true)
        .mode(crate::paths::constants::PRIVATE_FILE_MODE)
        .open(path)?;
    replacement.write_all(&encoded)?;
    replacement.sync_all()?;
    Ok(())
}
