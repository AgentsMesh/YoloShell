use super::constants::RUNTIME_PARTICIPANT;
use super::install_publication_fixture::with_fixture;

#[test]
fn post_exchange_retry_publishes_the_prepared_receipt() {
    with_fixture(|fixture| {
        fixture.exchange_runtime();
        super::install::install_runtime(fixture.paths, fixture.guard, fixture.loaded()).unwrap();
        assert!(
            super::receipt::read(
                fixture.guard,
                fixture.runtime,
                fixture.base.intent.transaction_id,
                RUNTIME_PARTICIPANT,
            )
            .unwrap()
            .is_some()
        );
    });
}

#[test]
fn receipt_publication_rejects_a_replaced_installed_target() {
    with_fixture(|fixture| {
        fixture.exchange_runtime();
        fixture
            .namespace
            .write_json(fixture.runtime, &fixture.base.intent.runtime.target_payload)
            .unwrap();
        assert!(
            super::install::install_runtime(fixture.paths, fixture.guard, fixture.loaded())
                .is_err()
        );
        fixture.assert_receipt_unpublished();
    });
}

#[test]
fn receipt_publication_rejects_a_replaced_displaced_source() {
    with_fixture(|fixture| {
        fixture.exchange_runtime();
        let stage = fixture.stage();
        let source: crate::model::RuntimeRecord =
            fixture.namespace.read_json(stage.path()).unwrap();
        let live =
            super::durable_io::content(&stage, fixture.base.intent.runtime.source_version).unwrap();
        stage
            .remove_if_content_version(live, crate::paths::constants::MAX_DURABLE_JSON_BYTES)
            .unwrap();
        fixture.namespace.write_json(stage.path(), &source).unwrap();
        assert!(
            super::install::install_runtime(fixture.paths, fixture.guard, fixture.loaded())
                .is_err()
        );
        fixture.assert_receipt_unpublished();
    });
}
