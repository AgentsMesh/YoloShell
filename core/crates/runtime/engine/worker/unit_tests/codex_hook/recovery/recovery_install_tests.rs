use super::recovery_test_support::RecoveryFixture;
use crate::worker::codex_hook::authority_registry::CodexAuthorityRegistry;
use crate::worker::codex_hook::publication::CodexAuthorityView;
use crate::worker::codex_hook::router::{
    CodexRecoveredRoute, CodexRetirementReadiness, CodexRouterError, CodexSessionRouter,
};
use crate::worker::codex_hook::runtime::CodexHookRuntime;
use std::sync::Arc;

const NO_ROUTES: usize = 0;
const SINGLE_ROUTE: usize = 1;
const VALIDATION_FAILURE: &str = "fixture recovery validation failed";

#[test]
fn duplicate_recovery_replays_without_rebuilding_runtime_authority() {
    let fixture = RecoveryFixture::new();
    let registry = Arc::new(CodexAuthorityRegistry::default());
    let router = CodexSessionRouter::new(
        fixture.paths.clone(),
        fixture.identity.terminal_id,
        fixture.identity.runtime_epoch,
    )
    .unwrap();
    let first = super::super::recovery_install::install(
        &router,
        &fixture.admission,
        |record| build_runtime(&fixture, Arc::clone(&registry), record),
        |_| Ok(()),
    )
    .unwrap();
    let first = first.accept();

    let replay = super::super::recovery_install::install(
        &router,
        &fixture.admission,
        |_| unreachable!(),
        |_| Ok(()),
    )
    .unwrap();
    let replay = replay.accept();

    assert!(Arc::ptr_eq(&first, &replay));
    assert!(matches!(
        first.runtime().subscribe_authority().borrow().clone(),
        CodexAuthorityView::Active(_)
    ));
}

#[test]
fn replay_validation_failure_preserves_the_committed_route() {
    let fixture = RecoveryFixture::new();
    let registry = Arc::new(CodexAuthorityRegistry::default());
    let router = router(&fixture, SINGLE_ROUTE);
    let first = super::super::recovery_install::install(
        &router,
        &fixture.admission,
        |record| build_runtime(&fixture, Arc::clone(&registry), record),
        |_| Ok(()),
    )
    .unwrap()
    .accept();

    let rejected = super::super::recovery_install::install(
        &router,
        &fixture.admission,
        |_| unreachable!(),
        |_| anyhow::bail!(VALIDATION_FAILURE),
    );

    assert!(rejected.is_err());
    assert_eq!(router.len().unwrap(), SINGLE_ROUTE);
    assert!(Arc::ptr_eq(
        &first,
        &router.resolve(&fixture.identity).unwrap()
    ));
}

#[test]
fn capacity_failure_rolls_back_every_route_from_the_recovery_round() {
    let fixture = RecoveryFixture::new();
    let _second = fixture.additional_admission();
    let router = router(&fixture, SINGLE_ROUTE);
    let process = crate::process::ProcessIdentity {
        pid: fixture.identity.process.pid,
        start_time: fixture.identity.process.start_time,
    };
    let context = crate::worker::codex_hook::recovery::CodexRecoveryContext::new(
        &fixture.paths,
        Arc::new(CodexAuthorityRegistry::default()),
        &router,
        fixture.shell,
        super::process_support::stub_process_source(process),
    );

    assert!(
        context
            .recover_observed(crate::process::ProcessIdentityObservation::Matching)
            .is_err()
    );
    assert_eq!(router.len().unwrap(), NO_ROUTES);
}

#[test]
fn explicit_rollback_propagates_a_poisoned_router_lock() {
    let fixture = RecoveryFixture::new();
    let (router, recovered) = recovered_route(&fixture);
    router.poison_lock_for_test();

    assert_eq!(recovered.rollback().unwrap_err(), CodexRouterError::Lock);
}

#[test]
fn explicit_rollback_propagates_a_retirement_conflict() {
    let fixture = RecoveryFixture::new();
    let (router, recovered) = recovered_route(&fixture);
    let CodexRetirementReadiness::Ready(permit) =
        router.begin_retirement(recovered.entry()).unwrap()
    else {
        unreachable!();
    };

    assert_eq!(
        recovered.rollback().unwrap_err(),
        CodexRouterError::Conflict
    );
    drop(permit);
    assert_eq!(router.len().unwrap(), SINGLE_ROUTE);
}

fn recovered_route(fixture: &RecoveryFixture) -> (CodexSessionRouter, CodexRecoveredRoute) {
    let registry = Arc::new(CodexAuthorityRegistry::default());
    let router = router(fixture, SINGLE_ROUTE);
    let recovered = super::super::recovery_install::install(
        &router,
        &fixture.admission,
        |record| build_runtime(fixture, registry, record),
        |_| Ok(()),
    )
    .unwrap();
    (router, recovered)
}

fn router(fixture: &RecoveryFixture, capacity: usize) -> CodexSessionRouter {
    CodexSessionRouter::with_capacity_limit(
        fixture.paths.clone(),
        fixture.identity.terminal_id,
        fixture.identity.runtime_epoch,
        capacity,
    )
    .unwrap()
}

pub(super) fn build_runtime(
    fixture: &RecoveryFixture,
    registry: Arc<CodexAuthorityRegistry>,
    record: &crate::codex_hooks::CodexAdmissionRecord,
) -> anyhow::Result<Arc<CodexHookRuntime>> {
    let owner = record.request().identity.clone();
    let process = crate::process::ProcessIdentity {
        pid: owner.process.pid,
        start_time: owner.process.start_time,
    };
    CodexHookRuntime::with_process_source(
        &fixture.paths,
        registry,
        owner.terminal_id,
        owner.runtime_epoch,
        fixture.shell,
        owner,
        record.request().provider_version,
        super::process_support::stub_process_source(process),
    )
    .map(|(runtime, _)| runtime)
}
