use super::guard::SessionDurableMutationGuard;
use super::preparation_schema::PreparationAuthority;
use crate::namespace_lease::NamespaceLease;
use crate::paths::Paths;
use std::io::Write;
use std::os::unix::fs::OpenOptionsExt;

const NAMESPACE_LABEL: &str = "durable reboot preparation retirement test";

#[test]
fn active_retirement_rejects_a_recreated_helper() {
    let fixture = super::validation::test_fixture::Fixture::new();
    let paths = Paths::new(fixture._root.path());
    let terminal_id = fixture.intent.terminal_id;
    let session = paths.session_dir(terminal_id);
    let namespace = NamespaceLease::capture(&[(NAMESPACE_LABEL, &session)]).unwrap();
    let guard = SessionDurableMutationGuard::acquire(&paths, &namespace, terminal_id).unwrap();
    let stage = super::staging::entry_for_cleanup(
        &guard,
        &paths.runtime_record(terminal_id),
        fixture.intent.transaction_id,
        super::constants::RUNTIME_PARTICIPANT,
    )
    .unwrap();
    let stage_path = stage.path().to_path_buf();
    let payload = fixture.intent.runtime.target_payload.clone();
    let _hook = super::prepare_test_hook::install_before_preparation_retire(move || {
        write_private_json(&stage_path, &payload)
    });

    assert!(
        super::preparation_recovery::recover_expected_orphan(
            &paths,
            &guard,
            fixture.intent.preparation_authority_version,
        )
        .is_err()
    );
    assert!(!stage.is_missing().unwrap());
    assert_authority_state(&paths, &guard, |value| {
        matches!(value, PreparationAuthority::Active(_))
    });
}

#[test]
fn settled_retirement_rejects_a_same_json_target_replacement() {
    let fixture = super::validation::test_fixture::Fixture::new();
    let paths = Paths::new(fixture._root.path());
    let terminal_id = fixture.intent.terminal_id;
    let session = paths.session_dir(terminal_id);
    let namespace = NamespaceLease::capture(&[(NAMESPACE_LABEL, &session)]).unwrap();
    let guard = SessionDurableMutationGuard::acquire(&paths, &namespace, terminal_id).unwrap();
    let loaded = super::publish_intent(&paths, &guard, &fixture.intent).unwrap();
    let runtime_path = paths.runtime_record(terminal_id);
    let runtime = fixture.intent.runtime.target_payload.clone();
    let transaction_id = fixture.intent.transaction_id;
    let replaced = std::rc::Rc::new(std::cell::Cell::new(false));
    let observed = replaced.clone();
    let _hook = super::prepare_test_hook::install_before_preparation_retire(move || {
        std::fs::remove_file(&runtime_path)?;
        write_private_json(&runtime_path, &runtime)?;
        let file = std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .open(&runtime_path)?;
        super::publication_marker::install(&file, transaction_id)?;
        observed.set(true);
        Ok(())
    });

    assert!(super::settlement::settle_pending_session_locked(&paths, &guard).is_err());
    assert!(replaced.get());
    assert!(
        namespace
            .file_is_missing(&paths.reboot_migration_intent(terminal_id))
            .unwrap()
    );
    assert_authority_state(&paths, &guard, |value| {
        matches!(value, PreparationAuthority::Settled(_))
    });
    drop(loaded);
}

#[test]
fn pending_authority_obeys_the_session_lifecycle_lock() {
    let fixture = super::validation::test_fixture::Fixture::new();
    let paths = Paths::new(fixture._root.path());
    let terminal_id = fixture.intent.terminal_id;
    let session = paths.session_dir(terminal_id);
    let namespace = NamespaceLease::capture(&[(NAMESPACE_LABEL, &session)]).unwrap();
    let _lifecycle = paths
        .try_acquire_session_lifecycle_lock(terminal_id)
        .unwrap()
        .unwrap();

    assert!(super::settlement::settle_pending_session(&paths, &namespace, terminal_id).is_err());
}

fn assert_authority_state(
    paths: &Paths,
    guard: &SessionDurableMutationGuard<'_>,
    expected: impl FnOnce(&PreparationAuthority) -> bool,
) {
    let loaded = super::preparation_authority::load_optional(paths, guard)
        .unwrap()
        .unwrap();
    assert!(expected(&loaded.value));
}

fn write_private_json(path: &std::path::Path, value: &impl serde::Serialize) -> anyhow::Result<()> {
    let bytes = serde_json::to_vec(value)?;
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(crate::paths::constants::PRIVATE_FILE_MODE)
        .open(path)?;
    file.write_all(&bytes)?;
    file.sync_all()?;
    Ok(())
}
