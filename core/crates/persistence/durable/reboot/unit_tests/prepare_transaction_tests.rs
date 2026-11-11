use super::guard::SessionDurableMutationGuard;
use super::schema::{OptionalParticipant, RebootMigrationIntent};
use super::validation::test_fixture::Fixture;
use crate::durable_reboot::durable_test_io::DurableTestRemoval;
use crate::namespace_lease::NamespaceLease;
use crate::paths::Paths;

const INJECTED_FAILURE: &str = "injected durable reboot preparation failure";
const NAMESPACE_LABEL: &str = "durable reboot prepare transaction test";
#[test]
fn stage_failure_rolls_back_every_created_helper() {
    with_clean_fixture(|fixture| {
        let calls = std::rc::Rc::new(std::cell::Cell::new(0_usize));
        let observed = calls.clone();
        let _hook = super::prepare_test_hook::install_after_stage(move || {
            let call = observed.get() + 1;
            observed.set(call);
            (call < 2)
                .then_some(())
                .ok_or_else(|| anyhow::anyhow!(INJECTED_FAILURE))
        });
        assert!(super::prepare_intent(fixture.paths, fixture.guard, fixture.targets).is_err());
        assert_eq!(calls.get(), 2);
        assert_no_transaction_helpers(fixture.paths, fixture.intent.terminal_id);
    });
}
#[test]
fn intent_publish_failure_rolls_back_the_complete_preparation() {
    with_clean_fixture(|fixture| {
        let _hook = super::prepare_test_hook::install_before_intent_commit(|| {
            Err(anyhow::anyhow!(INJECTED_FAILURE))
        });
        assert!(
            super::prepare_and_publish_intent(fixture.paths, fixture.guard, fixture.targets)
                .is_err()
        );
        assert!(
            fixture
                .namespace
                .file_is_missing(
                    &fixture
                        .paths
                        .reboot_migration_intent(fixture.intent.terminal_id)
                )
                .unwrap()
        );
        assert_no_transaction_helpers(fixture.paths, fixture.intent.terminal_id);
    });
}
#[test]
fn post_commit_error_retains_the_intent_and_preparation() {
    with_clean_fixture(|fixture| {
        let _hook = super::prepare_test_hook::install_after_intent_commit(|| {
            Err(anyhow::anyhow!(INJECTED_FAILURE))
        });
        assert!(
            super::prepare_and_publish_intent(fixture.paths, fixture.guard, fixture.targets)
                .is_err()
        );
        assert!(
            !fixture
                .namespace
                .file_is_missing(
                    &fixture
                        .paths
                        .reboot_migration_intent(fixture.intent.terminal_id)
                )
                .unwrap()
        );
        assert_transaction_helpers_present(fixture.paths, fixture.intent.terminal_id);
    });
}
struct CleanFixture<'a> {
    paths: &'a Paths,
    namespace: &'a NamespaceLease,
    guard: &'a SessionDurableMutationGuard<'a>,
    intent: &'a RebootMigrationIntent,
    targets: super::ParticipantTargets,
}
fn with_clean_fixture(test: impl for<'a> FnOnce(CleanFixture<'a>)) {
    let fixture = Fixture::new();
    let paths = Paths::new(fixture._root.path());
    let session = paths.session_dir(fixture.intent.terminal_id);
    let namespace = NamespaceLease::capture(&[(NAMESPACE_LABEL, &session)]).unwrap();
    discard_intent_helpers(&paths, &namespace, &fixture.intent);
    let guard = SessionDurableMutationGuard::acquire(&paths, &namespace, fixture.intent.terminal_id)
        .unwrap();
    let targets = super::validation::test_fixture::targets_from_intent(&fixture.intent);
    test(CleanFixture {
        paths: &paths,
        namespace: &namespace,
        guard: &guard,
        intent: &fixture.intent,
        targets,
    });
}
fn discard_intent_helpers(
    paths: &Paths,
    namespace: &NamespaceLease,
    intent: &RebootMigrationIntent,
) {
    let guard = SessionDurableMutationGuard::acquire(paths, namespace, intent.terminal_id).unwrap();
    let participants =
        super::participant_paths::ParticipantPaths::new(paths, intent.terminal_id).unwrap();
    remove_optional(
        &guard,
        &participants.startup_checkpoint,
        intent,
        super::constants::STARTUP_CHECKPOINT_PARTICIPANT,
        &intent.startup_checkpoint,
    );
    remove_optional(
        &guard,
        &participants.claude_custody,
        intent,
        super::constants::CLAUDE_CUSTODY_PARTICIPANT,
        &intent.claude_custody,
    );
    remove_optional(
        &guard,
        &participants.claude_generation,
        intent,
        super::constants::CLAUDE_GENERATION_PARTICIPANT,
        &intent.claude_generation,
    );
    remove_optional(
        &guard,
        &participants.pty_custody,
        intent,
        super::constants::PTY_CUSTODY_PARTICIPANT,
        &intent.pty_custody,
    );
    remove_required(
        &guard,
        &participants.runtime,
        intent,
        super::constants::RUNTIME_PARTICIPANT,
        &intent.runtime,
    );
}
fn remove_optional<T>(
    guard: &SessionDurableMutationGuard<'_>,
    path: &std::path::Path,
    intent: &RebootMigrationIntent,
    name: &str,
    participant: &OptionalParticipant<T>,
) {
    if let OptionalParticipant::Replace(required) = participant {
        remove_required(guard, path, intent, name, required);
    }
}
fn remove_required<T>(
    guard: &SessionDurableMutationGuard<'_>,
    path: &std::path::Path,
    intent: &RebootMigrationIntent,
    name: &str,
    required: &super::schema::RequiredParticipant<T>,
) {
    super::staging::entry_for_cleanup(guard, path, intent.transaction_id, name)
        .unwrap()
        .remove_if_durable_exact(
            required.target_version,
            crate::paths::constants::MAX_DURABLE_JSON_BYTES,
        )
        .unwrap();
    super::receipt::prepared_path(guard, path, intent.transaction_id, name)
        .unwrap()
        .remove_if_durable_exact(
            required.prepared_receipt_version,
            crate::paths::constants::MAX_DURABLE_JSON_BYTES,
        )
        .unwrap();
}
fn assert_no_transaction_helpers(paths: &Paths, terminal_id: uuid::Uuid) {
    assert_eq!(transaction_helper_count(paths, terminal_id), 0);
}
fn assert_transaction_helpers_present(paths: &Paths, terminal_id: uuid::Uuid) {
    assert!(transaction_helper_count(paths, terminal_id) > 0);
}
fn transaction_helper_count(paths: &Paths, terminal_id: uuid::Uuid) -> usize {
    let prefixes = [
        super::constants::REBOOT_STAGE_PREFIX,
        super::constants::REBOOT_RECEIPT_PREPARED_PREFIX,
        super::constants::REBOOT_RECEIPT_INSTALLED_PREFIX,
    ];
    std::fs::read_dir(paths.session_dir(terminal_id))
        .unwrap()
        .filter_map(|entry| {
            let name = entry.unwrap().file_name();
            let name = name.to_string_lossy();
            prefixes
                .iter()
                .any(|prefix| name.contains(prefix))
                .then_some(())
        })
        .count()
}
