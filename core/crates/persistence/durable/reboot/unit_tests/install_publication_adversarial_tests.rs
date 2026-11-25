use super::constants::{RUNTIME_PARTICIPANT, STARTUP_CHECKPOINT_PARTICIPANT};
use super::install_publication_fixture::PublicationFixture;
use super::install_publication_fixture::{with_fixture, with_unpublished_fixture};
use crate::durable_reboot::durable_test_io::DurableTestRemoval;
use crate::paths::constants::MAX_DURABLE_JSON_BYTES;
#[test]
fn same_content_prepared_receipt_replacement_is_rejected() {
    with_fixture(|fixture| {
        let prepared = super::receipt::prepared_path(
            fixture.guard,
            fixture.runtime,
            fixture.base.intent.transaction_id,
            RUNTIME_PARTICIPANT,
        )
        .unwrap();
        let receipt: super::receipt::InstallReceipt =
            fixture.namespace.read_json(prepared.path()).unwrap();
        prepared
            .remove_if_durable_exact(
                fixture.base.intent.runtime.prepared_receipt_version,
                MAX_DURABLE_JSON_BYTES,
            )
            .unwrap();
        fixture
            .namespace
            .write_json(prepared.path(), &receipt)
            .unwrap();

        assert!(install_runtime(&fixture).is_err());
        fixture.assert_receipt_unpublished();
    });
}

#[test]
fn same_content_installed_receipt_replacement_is_rejected() {
    with_fixture(|fixture| {
        fixture.install_runtime();
        let (receipt, identity) = super::receipt::read(
            fixture.guard,
            fixture.runtime,
            fixture.base.intent.transaction_id,
            RUNTIME_PARTICIPANT,
        )
        .unwrap()
        .unwrap();
        let installed = super::receipt::path(
            fixture.guard,
            fixture.runtime,
            fixture.base.intent.transaction_id,
            RUNTIME_PARTICIPANT,
        )
        .unwrap();
        installed
            .remove_if_durable_exact(identity, MAX_DURABLE_JSON_BYTES)
            .unwrap();
        fixture
            .namespace
            .write_json(installed.path(), &receipt)
            .unwrap();

        assert!(install_runtime(&fixture).is_err());
    });
}

#[test]
fn receipt_clear_preserves_a_same_json_replacement_inode() {
    with_fixture(|fixture| {
        fixture.install_runtime();
        let (receipt, identity) = super::receipt::read(
            fixture.guard,
            fixture.runtime,
            fixture.base.intent.transaction_id,
            RUNTIME_PARTICIPANT,
        )
        .unwrap()
        .unwrap();
        let installed = super::receipt::path(
            fixture.guard,
            fixture.runtime,
            fixture.base.intent.transaction_id,
            RUNTIME_PARTICIPANT,
        )
        .unwrap();
        installed
            .remove_if_durable_exact(identity, MAX_DURABLE_JSON_BYTES)
            .unwrap();
        fixture
            .namespace
            .write_json(installed.path(), &receipt)
            .unwrap();

        assert!(
            super::receipt::clear(
                fixture.guard,
                fixture.runtime,
                fixture.base.intent.transaction_id,
                RUNTIME_PARTICIPANT,
                fixture.base.intent.runtime.prepared_receipt_version,
                || super::intent_io::verify_exact(fixture.paths, fixture.guard, fixture.loaded()),
            )
            .is_err()
        );
        assert!(!installed.is_missing().unwrap());
    });
}

#[test]
fn installed_receipt_does_not_authorize_a_rolled_back_target() {
    with_fixture(|fixture| {
        fixture.exchange_runtime();
        let source: crate::model::RuntimeRecord =
            fixture.namespace.read_json(fixture.stage().path()).unwrap();
        fixture.install_runtime();
        fixture
            .namespace
            .write_json(fixture.runtime, &source)
            .unwrap();

        assert!(install_runtime(&fixture).is_err());
    });
}

#[test]
fn stale_same_payload_stage_from_another_transaction_is_rejected() {
    with_unpublished_fixture(|fixture| {
        let stale_payload: crate::model::RuntimeRecord =
            fixture.namespace.read_json(fixture.stage().path()).unwrap();
        let targets = super::validation::test_fixture::targets_from_intent(&fixture.base.intent);
        let intent = super::prepare_intent(fixture.paths, fixture.guard, targets).unwrap();
        let loaded = super::publish_intent(fixture.paths, fixture.guard, &intent).unwrap();
        super::install::install_prerequisites(fixture.paths, fixture.guard, &loaded).unwrap();
        let stale = fixture.stage();
        let current = fixture.stage_for(&intent);
        fixture
            .namespace
            .write_json(stale.path(), &stale_payload)
            .unwrap();
        current
            .remove_if_durable_exact(intent.runtime.target_version, MAX_DURABLE_JSON_BYTES)
            .unwrap();
        std::fs::rename(stale.path(), current.path()).unwrap();

        assert!(super::install::install_runtime(fixture.paths, fixture.guard, &loaded).is_err());
        assert!(
            super::receipt::read(
                fixture.guard,
                fixture.runtime,
                intent.transaction_id,
                RUNTIME_PARTICIPANT,
            )
            .unwrap()
            .is_none()
        );
    });
}

#[test]
fn target_replacement_after_participant_cleanup_is_rejected() {
    with_fixture(|fixture| {
        fixture.install_runtime();
        let checkpoint = fixture
            .paths
            .worker_startup_checkpoint(fixture.base.intent.terminal_id);
        let super::OptionalParticipant::Replace(required) = &fixture.base.intent.startup_checkpoint
        else {
            unreachable!();
        };
        super::staging::cleanup(
            fixture.guard,
            &checkpoint,
            fixture.base.intent.transaction_id,
            STARTUP_CHECKPOINT_PARTICIPANT,
            required.source_version,
        )
        .unwrap();
        super::receipt::clear(
            fixture.guard,
            &checkpoint,
            fixture.base.intent.transaction_id,
            STARTUP_CHECKPOINT_PARTICIPANT,
            required.prepared_receipt_version,
            || super::intent_io::verify_exact(fixture.paths, fixture.guard, fixture.loaded()),
        )
        .unwrap();
        fixture
            .namespace
            .write_json(&checkpoint, &required.target_payload)
            .unwrap();

        assert!(
            super::install::install_prerequisites(fixture.paths, fixture.guard, fixture.loaded(),)
                .is_err()
        );
    });
}

fn install_runtime(fixture: &PublicationFixture<'_>) -> anyhow::Result<()> {
    super::install::install_runtime(fixture.paths, fixture.guard, fixture.loaded())
}
