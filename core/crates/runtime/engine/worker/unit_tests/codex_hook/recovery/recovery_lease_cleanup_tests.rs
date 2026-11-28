use super::recovery_install_tests::build_runtime;
use super::recovery_test_support::RecoveryFixture;
use super::runtime_retirement_intake_test_support::replace_same_contents;
use crate::worker::codex_hook::authority_registry::CodexAuthorityRegistry;
use crate::worker::codex_hook::router::{
    CodexRouterError, CodexSessionRouter, DurableCodexAdmission,
};
use std::sync::Arc;

const SINGLE_ROUTE: usize = 1;
const ADMISSION_REPLACEMENT: &str = "recovery-lease-admission-replacement";

#[test]
fn admission_postflight_propagates_abort_lock_failure() {
    let fixture = RecoveryFixture::new();
    let router = router(&fixture);
    let path = fixture.admission.path().to_path_buf();
    let result = recover(&fixture, &router, |_| {
        replace_same_contents(&path, ADMISSION_REPLACEMENT);
        router.poison_lock_for_test();
        Ok(())
    });

    assert!(matches!(result, Err(CodexRouterError::Lock)));
}

#[test]
fn settlement_propagates_explicit_abort_failure() {
    let fixture = RecoveryFixture::new();
    let router = router(&fixture);
    router.fail_next_settle_for_test(CodexRouterError::Conflict);
    router.fail_next_abort_for_test(CodexRouterError::Lock);

    let result = recover(&fixture, &router, |_| Ok(()));

    assert!(matches!(result, Err(CodexRouterError::Lock)));
    assert_eq!(router.len().unwrap(), 0);
}

fn recover<Validate>(
    fixture: &RecoveryFixture,
    router: &CodexSessionRouter,
    validate: Validate,
) -> Result<crate::worker::codex_hook::router::CodexRecoveredRoute, CodexRouterError>
where
    Validate:
        FnOnce(&crate::worker::codex_hook::router::CodexRouteEntry) -> Result<(), CodexRouterError>,
{
    let admission = DurableCodexAdmission::from_authority(fixture.admission.clone()).unwrap();
    let registry = Arc::new(CodexAuthorityRegistry::default());
    router.recover_committed(
        admission,
        |record| build_runtime(fixture, registry, record).map_err(|_| CodexRouterError::Runtime),
        validate,
    )
}

fn router(fixture: &RecoveryFixture) -> CodexSessionRouter {
    CodexSessionRouter::with_capacity_limit(
        fixture.paths.clone(),
        fixture.identity.terminal_id,
        fixture.identity.runtime_epoch,
        SINGLE_ROUTE,
    )
    .unwrap()
}
