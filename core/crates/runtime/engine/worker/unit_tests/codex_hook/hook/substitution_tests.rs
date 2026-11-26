use super::support::*;
use crate::codex_hooks::{OutboxPostOperationFault, inject_outbox_fault};

const OUTBOX_FAULTS: [OutboxPostOperationFault; 3] = [
    OutboxPostOperationFault::TargetRemoved,
    OutboxPostOperationFault::LockRemoved,
    OutboxPostOperationFault::DirectoryMoved,
];

#[test]
fn reconcile_substitution_revokes_authority() {
    for fault in OUTBOX_FAULTS {
        let fixture = Fixture::new();
        fixture.enqueue(fixture.start(UNASSIGNED_EVENT_SEQUENCE));
        let authority = fixture.runtime.subscribe_authority();
        inject_outbox_fault(&fixture.runtime.outbox_path(), fault);

        assert!(fixture.runtime.reconcile_pending().is_err());

        assert_contained_view(&authority, fault);
    }
}

#[test]
fn process_exit_substitution_revokes_authority() {
    for fault in OUTBOX_FAULTS {
        let fixture = Fixture::new();
        fixture.enqueue(fixture.start(UNASSIGNED_EVENT_SEQUENCE));
        let authority = fixture.runtime.subscribe_authority();
        inject_outbox_fault(&fixture.runtime.outbox_path(), fault);
        let observed = crate::agent::hooks::ProcessIdentity::new(
            fixture.process.pid,
            fixture.process.start_time,
        );

        let result = fixture.runtime.observe_process_exit(observed);

        match fault {
            OutboxPostOperationFault::DirectoryMoved => assert!(matches!(
                result,
                Err(super::super::error::CodexHookRejection::AuthorityLost(_))
            )),
            OutboxPostOperationFault::TargetRemoved | OutboxPostOperationFault::LockRemoved => {
                assert!(matches!(
                    result,
                    Err(super::super::error::CodexHookRejection::Storage(_))
                ));
            }
        }
        assert_contained_view(&authority, fault);
    }
}

#[test]
fn startup_replay_substitution_rejects_runtime() {
    for fault in OUTBOX_FAULTS {
        let fixture = Fixture::new();
        fixture.enqueue(fixture.start(UNASSIGNED_EVENT_SEQUENCE));
        inject_outbox_fault(&fixture.runtime.outbox_path(), fault);

        assert!(fixture.try_restart_runtime().is_err());
    }
}

fn assert_contained_view(
    authority: &tokio::sync::watch::Receiver<super::super::publication::CodexAuthorityView>,
    fault: OutboxPostOperationFault,
) {
    let view = authority.borrow().clone();
    match fault {
        OutboxPostOperationFault::DirectoryMoved => {
            assert_eq!(
                view,
                super::super::publication::CodexAuthorityView::AuthorityLost
            );
        }
        OutboxPostOperationFault::TargetRemoved | OutboxPostOperationFault::LockRemoved => {
            assert_eq!(
                view,
                super::super::publication::CodexAuthorityView::StorageUnavailable
            );
        }
    }
}
