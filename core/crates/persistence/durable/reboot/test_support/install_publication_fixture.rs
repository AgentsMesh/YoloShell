use super::constants::RUNTIME_PARTICIPANT;
use super::guard::SessionDurableMutationGuard;
use super::schema::RebootMigrationIntent;
use super::validation::test_fixture::Fixture;
use crate::namespace_lease::NamespaceLease;
use crate::paths::Paths;

const NAMESPACE_LABEL: &str = "durable reboot receipt publication test";

pub(super) struct PublicationFixture<'a> {
    pub(super) base: &'a Fixture,
    pub(super) paths: &'a Paths,
    pub(super) namespace: &'a NamespaceLease,
    pub(super) guard: &'a SessionDurableMutationGuard<'a>,
    pub(super) runtime: &'a std::path::Path,
    pub(super) loaded: Option<super::intent_io::LoadedIntent>,
}

impl PublicationFixture<'_> {
    pub(super) fn exchange_runtime(&self) {
        super::staging::install(
            self.guard,
            self.runtime,
            self.base.intent.transaction_id,
            RUNTIME_PARTICIPANT,
            self.base.intent.runtime.source_version,
            self.base.intent.runtime.target_version,
            &self.base.intent.runtime.target_payload,
            || Ok(()),
        )
        .unwrap();
    }

    pub(super) fn install_runtime(&self) {
        super::install::install_runtime(self.paths, self.guard, self.loaded()).unwrap();
    }

    pub(super) fn loaded(&self) -> &super::intent_io::LoadedIntent {
        self.loaded.as_ref().expect("fixture intent is unpublished")
    }

    pub(super) fn assert_receipt_unpublished(&self) {
        assert!(
            super::receipt::read(
                self.guard,
                self.runtime,
                self.base.intent.transaction_id,
                RUNTIME_PARTICIPANT,
            )
            .unwrap()
            .is_none()
        );
    }

    pub(super) fn stage(&self) -> crate::paths::private_file::PrivateFileEntry {
        self.stage_for(&self.base.intent)
    }

    pub(super) fn stage_for(
        &self,
        intent: &RebootMigrationIntent,
    ) -> crate::paths::private_file::PrivateFileEntry {
        super::staging::entry_for_test(
            self.guard,
            self.runtime,
            intent.transaction_id,
            RUNTIME_PARTICIPANT,
        )
        .unwrap()
    }
}

pub(super) fn with_fixture(test: impl for<'a> FnOnce(PublicationFixture<'a>)) {
    with_unpublished_fixture(|mut fixture| {
        let loaded =
            super::publish_intent(fixture.paths, fixture.guard, &fixture.base.intent).unwrap();
        super::install::install_prerequisites(fixture.paths, fixture.guard, &loaded).unwrap();
        fixture.loaded = Some(loaded);
        test(fixture);
    });
}

pub(super) fn with_unpublished_fixture(test: impl for<'a> FnOnce(PublicationFixture<'a>)) {
    let base = Fixture::new();
    let paths = Paths::new(base._root.path());
    let terminal_id = base.intent.terminal_id;
    let session = paths.session_dir(terminal_id);
    let namespace = NamespaceLease::capture(&[(NAMESPACE_LABEL, &session)]).unwrap();
    let guard = SessionDurableMutationGuard::acquire(&paths, &namespace, terminal_id).unwrap();
    let runtime = paths.runtime_record(terminal_id);
    test(PublicationFixture {
        base: &base,
        paths: &paths,
        namespace: &namespace,
        guard: &guard,
        runtime: &runtime,
        loaded: None,
    });
}
