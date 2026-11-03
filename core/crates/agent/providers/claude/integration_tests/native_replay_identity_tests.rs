use super::fixture::Fixture;
use crate::agent::AgentBinding;
use crate::paths::identity::FileIdentity;
use std::{cell::Cell, os::unix::fs::PermissionsExt, path::Path};

const NEXT_EVENT: u64 = crate::claude_hooks::INITIAL_CLAUDE_EVENT_SEQUENCE + 1;
const REPLACEMENT: &str = "native-replay-replacement.json";

fn identity(fixture: &Fixture, path: &Path) -> FileIdentity {
    fixture
        .namespace
        .read_json_with_identity::<serde_json::Value>(path)
        .unwrap()
        .1
}

fn replace(fixture: &Fixture, path: &Path, bytes: &[u8]) -> FileIdentity {
    let temporary = path.parent().unwrap().join(REPLACEMENT);
    std::fs::write(&temporary, bytes).unwrap();
    std::fs::set_permissions(
        &temporary,
        std::fs::Permissions::from_mode(crate::paths::constants::PRIVATE_FILE_MODE),
    )
    .unwrap();
    std::fs::rename(temporary, path).unwrap();
    identity(fixture, path)
}

#[test]
fn exact_accepted_replays_preserve_authority_and_binding_file_identity() {
    let fixture = Fixture::new();
    for request in [&fixture.start, &fixture.state_event(NEXT_EVENT)] {
        let record = fixture.apply(request).unwrap();
        fixture
            .store
            .publish_binding_in_namespace(&fixture.namespace, &record, || Ok(()))
            .unwrap();
        let binding = fixture.paths.agent_binding(request.scope.terminal_id);
        let authority_before = identity(&fixture, fixture.store.path());
        let binding_before = identity(&fixture, &binding);
        let replay = fixture.apply(request).unwrap();
        assert_eq!(replay, record);
        fixture
            .store
            .publish_binding_in_namespace(&fixture.namespace, &replay, || Ok(()))
            .unwrap();
        assert_eq!(identity(&fixture, fixture.store.path()), authority_before);
        assert_eq!(identity(&fixture, &binding), binding_before);
        assert_eq!(
            fixture
                .namespace
                .read_json::<AgentBinding>(&binding)
                .unwrap(),
            record.recovery_binding(true)
        );
    }
}

#[test]
fn replay_generation_rejects_same_content_replacement_from_its_commit_fence() {
    let fixture = Fixture::new();
    fixture.apply(&fixture.start).unwrap();
    let path = fixture.store.path();
    let previous = identity(&fixture, path);
    let bytes = std::fs::read(path).unwrap();
    let replaced = Cell::new(None);
    assert!(
        fixture
            .store
            .apply_in_namespace(
                &fixture.namespace,
                &fixture.start,
                fixture.worker,
                &fixture.boot_id,
                &fixture.evidence,
                || {
                    replaced.set(Some(replace(&fixture, path, &bytes)));
                    Ok(())
                }
            )
            .is_err()
    );
    assert_ne!(replaced.get(), Some(previous));
    assert_eq!(Some(identity(&fixture, path)), replaced.get());
}

#[test]
fn unchanged_binding_rechecks_both_source_and_target_identity_after_fence() {
    for source_changed in [false, true] {
        let fixture = Fixture::new();
        let record = fixture.apply(&fixture.start).unwrap();
        fixture
            .store
            .publish_binding_in_namespace(&fixture.namespace, &record, || Ok(()))
            .unwrap();
        let binding = fixture.paths.agent_binding(record.scope.terminal_id);
        let path = if source_changed {
            fixture.store.path()
        } else {
            binding.as_path()
        };
        let previous = identity(&fixture, path);
        let bytes = std::fs::read(path).unwrap();
        let replaced = Cell::new(None);
        assert!(
            fixture
                .store
                .publish_binding_in_namespace(&fixture.namespace, &record, || {
                    replaced.set(Some(replace(&fixture, path, &bytes)));
                    Ok(())
                })
                .is_err()
        );
        assert_ne!(replaced.get(), Some(previous));
        assert_eq!(Some(identity(&fixture, path)), replaced.get());
    }
}

#[test]
fn missing_binding_is_repaired_without_rewriting_authority_or_overwriting_raced_file() {
    for race in [false, true] {
        let fixture = Fixture::new();
        let record = fixture.apply(&fixture.start).unwrap();
        let binding = fixture.paths.agent_binding(record.scope.terminal_id);
        assert!(!binding.exists());
        let authority_before = identity(&fixture, fixture.store.path());
        let bytes = serde_json::to_vec(&record.recovery_binding(true)).unwrap();
        let replacement = Cell::new(None);
        let result =
            fixture
                .store
                .publish_binding_in_namespace(&fixture.namespace, &record, || {
                    if race && replacement.get().is_none() {
                        replacement.set(Some(replace(&fixture, &binding, &bytes)));
                    }
                    Ok(())
                });
        if race {
            assert!(result.is_err());
            assert_eq!(Some(identity(&fixture, &binding)), replacement.get());
        } else {
            result.unwrap();
            assert_eq!(
                fixture
                    .namespace
                    .read_json::<AgentBinding>(&binding)
                    .unwrap(),
                record.recovery_binding(true)
            );
            let established = identity(&fixture, &binding);
            fixture
                .store
                .publish_binding_in_namespace(&fixture.namespace, &record, || Ok(()))
                .unwrap();
            assert_eq!(identity(&fixture, &binding), established);
        }
        assert_eq!(identity(&fixture, fixture.store.path()), authority_before);
    }
}
