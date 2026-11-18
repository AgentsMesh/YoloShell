use super::{creation_support::Fixture, fixtures, initial_control_late_grants::pending_acquire};
use crate::{
    control, events,
    network::{Job, Update},
    polling,
};
use yolo_hub_client::ClientError;
use yolo_hub_client_channels::ControlError;
use yolo_hub_identity::{RuntimeEpoch, RuntimeTarget};
use yolo_hub_rpc_protocol::{Command, Reply};
use yolo_hub_terminal_protocol::{PublicError, TerminalSummary};
use yolo_presentation_layout::Axis;

enum Change {
    ReboundPane,
    ReplacedRuntime,
}

fn change_context(fixture: &mut Fixture, target: RuntimeTarget, change: Change) {
    match change {
        Change::ReboundPane => {
            // Keep the original terminal bound in the background, so pruning an
            // unbound policy cannot hide an incorrectly retained write intent.
            fixture
                .state
                .tab_layout_mut()
                .split(Axis::Horizontal, Some(target.address))
                .unwrap();
            let other = fixtures::session();
            let address = other.fields().address;
            fixture.state.terminals.insert(address, other);
            fixture
                .state
                .tab_layout_mut()
                .bind(fixture.pane, Some(address))
                .unwrap();
            fixture.state.tab_layout_mut().focus(fixture.pane).unwrap();
        }
        Change::ReplacedRuntime => {
            let mut fields = fixture.state.terminals[&target.address].fields().clone();
            fields.runtime.as_mut().unwrap().target.runtime_epoch = RuntimeEpoch::new();
            fixture
                .state
                .replace_terminals(vec![TerminalSummary::new(fields).unwrap()]);
        }
    }
}

#[test]
fn late_initial_success_with_valid_scope_cannot_install_a_writer_after_context_replacement() {
    for change in [Change::ReboundPane, Change::ReplacedRuntime] {
        let (mut fixture, request, grant) = pending_acquire();
        let target = grant.fields().target;
        let ui_epoch = fixture.state.ui_epoch;
        change_context(&mut fixture, target, change);
        assert_eq!(fixture.state.ui_epoch, ui_epoch);
        assert!(request.validity.as_ref().unwrap().is_current());
        events::apply(
            &mut fixture.state,
            &fixture.jobs,
            Update::Reply {
                command: request.command,
                reply: Reply::Lease { grant },
            },
        );
        assert!(fixture.state.writers.is_empty());
        assert_eq!(
            control::label(&fixture.state, target.address),
            crate::constants::READ_ONLY_LABEL
        );
        assert!(matches!(fixture.incoming.try_recv().unwrap(), Job::Call {
            command: Command::Release { grant: actual }, ..
        } if actual == grant));
        polling::renew_leases(&mut fixture.state, &fixture.jobs);
        assert!(fixture.incoming.try_recv().is_err());
    }
}

#[test]
fn late_initial_failure_with_valid_scope_cannot_promote_obsolete_context_to_write_intent() {
    for change in [Change::ReboundPane, Change::ReplacedRuntime] {
        let (mut fixture, request, grant) = pending_acquire();
        let target = grant.fields().target;
        let ui_epoch = fixture.state.ui_epoch;
        change_context(&mut fixture, target, change);
        assert_eq!(fixture.state.ui_epoch, ui_epoch);
        assert!(request.validity.as_ref().unwrap().is_current());
        let error = ControlError::Client(ClientError::Remote(PublicError::Conflict));
        // Exercise the promotion boundary directly, including the runtime case
        // normally also rejected by the outer channel's obsolete-target guard.
        control::failed(&mut fixture.state, &request, &error);
        events::apply(
            &mut fixture.state,
            &fixture.jobs,
            Update::Failed {
                command: Some(request.command),
                message: error.to_string(),
                connected: true,
                transient: false,
            },
        );
        assert!(fixture.state.writers.is_empty());
        assert_eq!(
            control::label(&fixture.state, target.address),
            crate::constants::READ_ONLY_LABEL
        );
        polling::renew_leases(&mut fixture.state, &fixture.jobs);
        assert!(fixture.incoming.try_recv().is_err());
    }
}
