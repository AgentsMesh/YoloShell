use super::constants::PTY_CUSTODY_PARTICIPANT;
use super::guard::SessionDurableMutationGuard;
use super::schema::OptionalParticipant;
use super::validation::rollover_fixture::RolloverFixture;
use crate::namespace_lease::NamespaceLease;
use crate::paths::Paths;

const NAMESPACE_LABEL: &str = "durable reboot PTY receipt publication test";

#[test]
fn post_exchange_retry_rechecks_the_domain_separated_source() {
    with_fixture(|fixture| {
        fixture.exchange_custody();
        super::install_pty::verify_source_object(
            fixture.guard,
            fixture.custody_path,
            &fixture.base.intent,
            fixture.required,
        )
        .unwrap();
    });
}

#[test]
fn replaced_displaced_custody_fails_the_receipt_fence() {
    with_fixture(|fixture| {
        fixture.exchange_custody();
        let stage = super::staging::entry_for_test(
            fixture.guard,
            fixture.custody_path,
            fixture.base.intent.transaction_id,
            PTY_CUSTODY_PARTICIPANT,
        )
        .unwrap();
        let source: crate::pty_custody::CustodyDurableRecord =
            fixture.namespace.read_json(stage.path()).unwrap();
        let live = super::durable_io::content(&stage, fixture.required.source_version).unwrap();
        stage
            .remove_if_content_version(live, crate::paths::constants::MAX_DURABLE_JSON_BYTES)
            .unwrap();
        fixture.namespace.write_json(stage.path(), &source).unwrap();

        assert!(
            super::install_pty::verify_source_object(
                fixture.guard,
                fixture.custody_path,
                &fixture.base.intent,
                fixture.required,
            )
            .is_err()
        );
    });
}

struct InstallPtyFixture<'a> {
    base: &'a RolloverFixture,
    namespace: &'a NamespaceLease,
    guard: &'a SessionDurableMutationGuard<'a>,
    custody_path: &'a std::path::Path,
    required: &'a super::schema::RequiredParticipant<crate::pty_custody::CustodyDurableRecord>,
}

impl InstallPtyFixture<'_> {
    fn exchange_custody(&self) {
        super::staging::install(
            self.guard,
            self.custody_path,
            self.base.intent.transaction_id,
            PTY_CUSTODY_PARTICIPANT,
            self.required.source_version,
            self.required.target_version,
            &self.required.target_payload,
            || Ok(()),
        )
        .unwrap();
    }
}

fn with_fixture(test: impl for<'a> FnOnce(InstallPtyFixture<'a>)) {
    let base = RolloverFixture::new();
    let paths = Paths::new(base._base._root.path());
    let terminal_id = base.intent.terminal_id;
    let session = paths.session_dir(terminal_id);
    let namespace = NamespaceLease::capture(&[(NAMESPACE_LABEL, &session)]).unwrap();
    let guard = SessionDurableMutationGuard::acquire(&paths, &namespace, terminal_id).unwrap();
    super::publish_intent(&paths, &guard, &base.intent).unwrap();
    let custody_path = paths.pty_custody(terminal_id);
    let OptionalParticipant::Replace(required) = &base.intent.pty_custody else {
        unreachable!();
    };
    test(InstallPtyFixture {
        base: &base,
        namespace: &namespace,
        guard: &guard,
        custody_path: &custody_path,
        required: &required,
    });
}
