use super::{
    control_recovery_support::*, initial_control_late_grants::pending_acquire,
    query_support::Fixture,
};
use crate::{
    control, events,
    network::{Job, Update},
    polling,
};
use uuid::Uuid;
use yolo_hub_client::ClientError;
use yolo_hub_client_channels::{ControlError, ControlEvent, ControlRequest};
use yolo_hub_rpc_protocol::{Command, Reply};
use yolo_hub_terminal_protocol::PublicError;

#[test]
fn exact_initial_acquire_failure_preserves_intent_and_classifies_the_failure() {
    for (error, label) in [
        (PublicError::Conflict, OCCUPIED_LABEL),
        (PublicError::Internal, RESTORING_LABEL),
        (PublicError::Unauthorized, DENIED_LABEL),
    ] {
        let (mut fixture, request, grant) = pending_acquire();
        events::apply(
            &mut fixture.state,
            &fixture.jobs,
            Update::Control(ControlEvent::Finished {
                request,
                result: Err(ControlError::Client(ClientError::Remote(error))),
                connected: true,
            }),
        );
        assert!(fixture.state.initial_control.is_none());
        assert!(fixture.state.writers.is_empty());
        assert_eq!(
            control::label(&fixture.state, grant.fields().target.address),
            label
        );
        polling::renew_leases(&mut fixture.state, &fixture.jobs);
        assert!(fixture.incoming.try_recv().is_err());
    }
}

#[test]
fn unrelated_initial_failure_does_not_cancel_or_promote_the_real_pending_request() {
    let (mut fixture, request, grant) = pending_acquire();
    let mut unrelated = request.clone();
    let Command::Lease { request: lease } = &mut unrelated.command else {
        panic!()
    };
    lease.request_id = Uuid::new_v4();
    control::failed(
        &mut fixture.state,
        &unrelated,
        &ControlError::Client(ClientError::Remote(PublicError::Unauthorized)),
    );
    assert!(request.validity.as_ref().unwrap().is_current());
    assert_eq!(
        control::label(&fixture.state, grant.fields().target.address),
        crate::constants::READ_ONLY_LABEL
    );
    super::initial_control_late_grants::finish(&mut fixture, request, grant);
    assert_eq!(
        fixture.state.writers[&grant.fields().target.address].grant,
        grant
    );
    assert!(fixture.incoming.try_recv().is_err());
}

#[test]
fn failed_current_renewal_recovers_conflict_but_requires_explicit_intent_after_denial() {
    for (error, label) in [
        (PublicError::Conflict, RESTORING_LABEL),
        (PublicError::Unauthorized, DENIED_LABEL),
    ] {
        let mut fixture = Fixture::new();
        let grant = fixture.grant;
        let request = ControlRequest {
            generation: fixture.state.control_generation,
            destination: fixture.target.address.hub_id,
            command: Command::Renew { grant },
            validity: None,
        };
        fixture.finish(
            request,
            Err(ControlError::Client(ClientError::Remote(error))),
            true,
        );
        assert!(fixture.state.writers.is_empty());
        assert_eq!(
            control::label(&fixture.state, fixture.target.address),
            label
        );
        polling::renew_leases(&mut fixture.state, &fixture.jobs);
        fixture.no_jobs();
        control::acquire(&mut fixture.state, &fixture.jobs, fixture.target);
        let next = acquisition(&mut fixture);
        let grant = replacement(&fixture);
        fixture.finish(next, Ok(Reply::Lease { grant }), true);
        assert_eq!(fixture.state.writers[&fixture.target.address].grant, grant);
    }
}

#[test]
fn replacing_or_unbinding_a_terminal_retires_pending_acquisition_scope() {
    for replace in [false, true] {
        let mut fixture = Fixture::new();
        fixture.state.writers.clear();
        control::acquire(&mut fixture.state, &fixture.jobs, fixture.target);
        let old = acquisition(&mut fixture);
        if replace {
            let mut fields = fixture.state.terminals[&fixture.target.address]
                .fields()
                .clone();
            fields.runtime.as_mut().unwrap().target.runtime_epoch =
                yolo_hub_identity::RuntimeEpoch::new();
            let next = fields.runtime.unwrap().target;
            fixture.state.replace_terminals(vec![
                yolo_hub_terminal_protocol::TerminalSummary::new(fields).unwrap(),
            ]);
            assert!(!fixture.state.control.permits(next));
        } else {
            fixture.state.bind_focused(None).unwrap();
        }
        control::reconcile(&mut fixture.state, &fixture.jobs);
        assert!(!old.validity.as_ref().unwrap().is_current());
        assert!(fixture.state.writers.is_empty());
        fixture.no_jobs();
        assert!(!matches!(
            fixture.incoming.try_recv(),
            Ok(Job::Acquire { .. })
        ));
    }
}
