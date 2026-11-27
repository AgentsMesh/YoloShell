use super::*;
use crate::codex_hooks::constants::{
    ARTIFACTS_DIRECTORY_NAME, OUTBOX_FILE_NAME, SNAPSHOT_FILE_NAME, test_values,
};
#[path = "../test_support/artifacts.rs"]
mod support;
use support::*;

const FOREIGN_HOME_DIRECTORY: &str = "foreign";
const OUTSIDE_FILE_NAME: &str = "outside.json";
const PARENT_DIRECTORY_COMPONENT: &str = "..";
const OUTSIDE_SENTINEL: &[u8] = b"outside";
#[test]
fn artifact_paths_are_derived_from_the_trusted_home() {
    let fixture = Fixture::new();
    let expected = fixture
        .paths
        .session_dir(fixture.session_id)
        .join(ARTIFACTS_DIRECTORY_NAME)
        .join(fixture.runtime_epoch.to_string())
        .join(fixture.invocation_id.to_string());

    assert_eq!(fixture.artifacts.directory(), expected);
    assert_eq!(
        fixture.artifacts.outbox_path(),
        expected.join(OUTBOX_FILE_NAME)
    );
    assert_eq!(
        fixture.artifacts.snapshot_path(),
        expected.join(SNAPSHOT_FILE_NAME)
    );
}

#[test]
fn separate_invocations_never_share_artifacts() {
    let fixture = Fixture::new();
    let second = CodexHookArtifacts::new(
        &fixture.paths,
        fixture.session_id,
        fixture.runtime_epoch,
        Uuid::new_v4(),
    )
    .unwrap();

    assert_ne!(fixture.artifacts.directory(), second.directory());
    assert_ne!(fixture.artifacts.outbox_path(), second.outbox_path());
    assert_ne!(fixture.artifacts.snapshot_path(), second.snapshot_path());
}

#[test]
fn invalid_home_and_nil_identifiers_are_rejected() {
    let fixture = Fixture::new();
    let relative = crate::paths::Paths::new(test_values::HOME_DIRECTORY);
    let traversal = crate::paths::Paths::new(fixture.root.path().join(PARENT_DIRECTORY_COMPONENT));

    assert!(
        CodexHookArtifacts::new(
            &relative,
            fixture.session_id,
            fixture.runtime_epoch,
            fixture.invocation_id,
        )
        .is_err()
    );
    assert!(
        CodexHookArtifacts::new(
            &traversal,
            fixture.session_id,
            fixture.runtime_epoch,
            fixture.invocation_id,
        )
        .is_err()
    );
    for (session_id, runtime_epoch, invocation_id) in [
        (Uuid::nil(), fixture.runtime_epoch, fixture.invocation_id),
        (fixture.session_id, Uuid::nil(), fixture.invocation_id),
        (fixture.session_id, fixture.runtime_epoch, Uuid::nil()),
    ] {
        assert!(
            CodexHookArtifacts::new(&fixture.paths, session_id, runtime_epoch, invocation_id)
                .is_err()
        );
    }
}

#[test]
fn identity_has_no_api_for_a_foreign_outbox_root() {
    let fixture = Fixture::new();
    let identity = CodexHookIdentity::for_test(
        &fixture.paths,
        fixture.identity(),
        ACCEPTED_PROVIDER_VERSION,
    )
    .unwrap();
    let foreign = fixture
        .root
        .path()
        .join(FOREIGN_HOME_DIRECTORY)
        .join(identity.hook.terminal_id.to_string());

    assert!(identity.outbox_path.starts_with(fixture.paths.home()));
    assert!(!identity.outbox_path.starts_with(foreign));
}

#[cfg(unix)]
#[test]
fn directory_artifact_components_never_follow_symlinks() {
    assert_directory_link_rejected(LinkComponent::Session);
    assert_directory_link_rejected(LinkComponent::Artifacts);
    assert_directory_link_rejected(LinkComponent::Runtime);
    assert_directory_link_rejected(LinkComponent::Invocation);
}

#[cfg(unix)]
#[test]
fn artifact_files_never_follow_symlinks() {
    let fixture = Fixture::new();
    fixture.paths.ensure_session(fixture.session_id).unwrap();
    private_directory(fixture.artifacts.directory());
    let outside = tempfile::tempdir().unwrap();

    let outbox_target = outside.path().join(OUTSIDE_FILE_NAME);
    std::fs::write(&outbox_target, OUTSIDE_SENTINEL).unwrap();
    std::os::unix::fs::symlink(&outbox_target, fixture.artifacts.outbox_path()).unwrap();
    assert_outbox_rejected(&fixture);
    assert_eq!(std::fs::read(&outbox_target).unwrap(), OUTSIDE_SENTINEL);

    std::os::unix::fs::symlink(&outbox_target, fixture.artifacts.snapshot_path()).unwrap();
    assert!(crate::paths::atomic_write_json(&fixture.artifacts.snapshot_path(), &true,).is_err());
    assert_eq!(std::fs::read(&outbox_target).unwrap(), OUTSIDE_SENTINEL);
}

#[cfg(unix)]
#[derive(Clone, Copy)]
enum LinkComponent {
    Session,
    Artifacts,
    Runtime,
    Invocation,
}

#[cfg(unix)]
fn assert_directory_link_rejected(component: LinkComponent) {
    let fixture = Fixture::new();
    fixture.paths.ensure_base().unwrap();
    let link = match component {
        LinkComponent::Session => fixture.paths.session_dir(fixture.session_id),
        LinkComponent::Artifacts => {
            fixture.paths.ensure_session(fixture.session_id).unwrap();
            fixture
                .paths
                .session_dir(fixture.session_id)
                .join(ARTIFACTS_DIRECTORY_NAME)
        }
        LinkComponent::Runtime => {
            fixture.paths.ensure_session(fixture.session_id).unwrap();
            let root = fixture
                .paths
                .session_dir(fixture.session_id)
                .join(ARTIFACTS_DIRECTORY_NAME);
            private_directory(&root);
            root.join(fixture.runtime_epoch.to_string())
        }
        LinkComponent::Invocation => {
            fixture.paths.ensure_session(fixture.session_id).unwrap();
            private_directory(fixture.artifacts.directory().parent().unwrap());
            fixture.artifacts.directory().to_path_buf()
        }
    };
    let outside = tempfile::tempdir().unwrap();
    std::os::unix::fs::symlink(outside.path(), link).unwrap();

    assert_outbox_rejected(&fixture);
    assert_eq!(std::fs::read_dir(outside.path()).unwrap().count(), 0);
}
