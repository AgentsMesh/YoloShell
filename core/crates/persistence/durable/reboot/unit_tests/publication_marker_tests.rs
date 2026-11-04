use super::guard::SessionDurableMutationGuard;
use super::install_publication_fixture::{with_fixture, with_unpublished_fixture};
use super::preparation_schema::PreparationAuthority;
use crate::namespace_lease::NamespaceLease;
use crate::paths::Paths;
use crate::paths::constants::MAX_DURABLE_JSON_BYTES;
use uuid::Uuid;

const NAMESPACE_LABEL: &str = "durable reboot publication marker test";
const PRIMARY_WRONG_TRANSACTION_BITS: u128 = u128::MAX;
const SECONDARY_WRONG_TRANSACTION_BITS: u128 = u128::MAX - 1;

#[test]
fn prepared_target_is_bound_to_its_transaction_before_identity_capture() {
    with_unpublished_fixture(|fixture| {
        let stage = fixture.stage();
        let opened = stage.open_read(MAX_DURABLE_JSON_BYTES).unwrap();
        assert_eq!(
            super::durable_io::capture(&stage).unwrap(),
            fixture.base.intent.runtime.target_version
        );
        super::publication_marker::verify(opened.file(), fixture.base.intent.transaction_id)
            .unwrap();
    });
}

#[test]
fn installed_retry_rejects_a_missing_publication_marker() {
    with_fixture(|fixture| {
        fixture.install_runtime();
        mutate_runtime_marker(&fixture, MarkerMutation::Remove);

        assert!(
            super::install::install_runtime(fixture.paths, fixture.guard, fixture.loaded())
                .is_err()
        );
    });
}

#[test]
fn installed_retry_rejects_another_transactions_marker() {
    with_fixture(|fixture| {
        fixture.install_runtime();
        mutate_runtime_marker(&fixture, MarkerMutation::WrongTransaction);

        assert!(
            super::install::install_runtime(fixture.paths, fixture.guard, fixture.loaded())
                .is_err()
        );
    });
}

#[test]
fn active_orphan_recovery_rejects_a_missing_stage_marker() {
    let fixture = super::validation::test_fixture::Fixture::new();
    let paths = Paths::new(fixture._root.path());
    let terminal_id = fixture.intent.terminal_id;
    let namespace =
        NamespaceLease::capture(&[(NAMESPACE_LABEL, &paths.session_dir(terminal_id))]).unwrap();
    let guard = SessionDurableMutationGuard::acquire(&paths, &namespace, terminal_id).unwrap();
    let stage = super::staging::entry_for_test(
        &guard,
        &paths.runtime_record(terminal_id),
        fixture.intent.transaction_id,
        super::constants::RUNTIME_PARTICIPANT,
    )
    .unwrap();
    let opened = stage.open_rw_or_create(MAX_DURABLE_JSON_BYTES).unwrap();
    super::publication_marker::remove(opened.file()).unwrap();

    assert!(
        super::preparation_recovery::recover_expected_orphan(
            &paths,
            &guard,
            fixture.intent.preparation_authority_version,
        )
        .is_err()
    );
    assert!(!stage.is_missing().unwrap());
    let authority = super::preparation_authority::load_optional(&paths, &guard)
        .unwrap()
        .unwrap();
    assert!(matches!(authority.value, PreparationAuthority::Active(_)));
}

#[test]
fn settled_retirement_rejects_a_missing_publication_marker() {
    assert_settled_retirement_rejects(MarkerMutation::Remove);
}

#[test]
fn settled_retirement_rejects_another_transactions_marker() {
    assert_settled_retirement_rejects(MarkerMutation::WrongTransaction);
}

#[derive(Clone, Copy)]
enum MarkerMutation {
    Remove,
    WrongTransaction,
}

impl MarkerMutation {
    fn apply(self, file: &std::fs::File, expected: Uuid) -> anyhow::Result<()> {
        match self {
            Self::Remove => super::publication_marker::remove(file),
            Self::WrongTransaction => {
                super::publication_marker::install(file, wrong_transaction(expected))
            }
        }
    }
}

fn mutate_runtime_marker(
    fixture: &super::install_publication_fixture::PublicationFixture<'_>,
    mutation: MarkerMutation,
) {
    let entry = fixture.guard.namespace().file(fixture.runtime).unwrap();
    let opened = entry.open_rw_or_create(MAX_DURABLE_JSON_BYTES).unwrap();
    mutation
        .apply(opened.file(), fixture.base.intent.transaction_id)
        .unwrap();
}

fn assert_settled_retirement_rejects(mutation: MarkerMutation) {
    let fixture = super::validation::test_fixture::Fixture::new();
    let paths = Paths::new(fixture._root.path());
    let terminal_id = fixture.intent.terminal_id;
    let namespace =
        NamespaceLease::capture(&[(NAMESPACE_LABEL, &paths.session_dir(terminal_id))]).unwrap();
    let guard = SessionDurableMutationGuard::acquire(&paths, &namespace, terminal_id).unwrap();
    let loaded = super::publish_intent(&paths, &guard, &fixture.intent).unwrap();
    let runtime_path = paths.runtime_record(terminal_id);
    let transaction_id = fixture.intent.transaction_id;
    let _hook = super::prepare_test_hook::install_before_preparation_retire(move || {
        let entry = crate::paths::private_file::PrivateFileEntry::open(&runtime_path)?;
        let opened = entry.open_rw_or_create(MAX_DURABLE_JSON_BYTES)?;
        mutation.apply(opened.file(), transaction_id)
    });

    assert!(super::settlement::settle_pending_session_locked(&paths, &guard).is_err());
    assert!(
        namespace
            .file_is_missing(&paths.reboot_migration_intent(terminal_id))
            .unwrap()
    );
    let authority = super::preparation_authority::load_optional(&paths, &guard)
        .unwrap()
        .unwrap();
    assert!(matches!(authority.value, PreparationAuthority::Settled(_)));
    drop(loaded);
}

fn wrong_transaction(expected: Uuid) -> Uuid {
    let primary = Uuid::from_u128(PRIMARY_WRONG_TRANSACTION_BITS);
    if primary != expected {
        primary
    } else {
        Uuid::from_u128(SECONDARY_WRONG_TRANSACTION_BITS)
    }
}
