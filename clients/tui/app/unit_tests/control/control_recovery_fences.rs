use super::{control_recovery_support::*, query_support::Fixture};
use crate::{control, events, network::Update, polling, terminal_actions};
use yolo_hub_client::ClientError;
use yolo_hub_client_channels::ControlError;
use yolo_hub_rpc_protocol::{Command, Reply};
use yolo_hub_terminal_protocol::PublicError;

#[test]
fn explicit_release_rejects_late_renewal_and_does_not_restart_acquisition() {
    let mut fixture = Fixture::new();
    release_key(&mut fixture);
    let grant = fixture.grant;
    released(&mut fixture, grant);
    assert!(!fixture.state.control.permits(fixture.target));
    events::apply(
        &mut fixture.state,
        &fixture.jobs,
        Update::Reply {
            command: Command::Renew { grant },
            reply: Reply::Lease { grant },
        },
    );
    fixture.no_jobs();
    assert!(fixture.state.writers.is_empty());
    elapsed_retry(&mut fixture);
    polling::renew_leases(&mut fixture.state, &fixture.jobs);
    fixture.no_jobs();
}

#[test]
fn explicit_release_cancels_pending_acquisition_and_releases_its_late_grant() {
    let mut fixture = Fixture::new();
    fixture.state.writers.clear();
    control::acquire(&mut fixture.state, &fixture.jobs, fixture.target);
    let pending = acquisition(&mut fixture);
    release_key(&mut fixture);
    assert!(!pending.validity.as_ref().unwrap().is_current());
    fixture.no_jobs();
    let grant = replacement(&fixture);
    fixture.finish(pending, Ok(Reply::Lease { grant }), true);
    released(&mut fixture, grant);
    assert!(fixture.state.writers.is_empty());
    polling::renew_leases(&mut fixture.state, &fixture.jobs);
    fixture.no_jobs();
}

#[test]
fn competing_holder_is_visible_without_takeover_and_denial_stops_automatic_retry() {
    for (error, label) in [
        (ClientError::Remote(PublicError::Conflict), OCCUPIED_LABEL),
        (ClientError::Remote(PublicError::Unauthorized), DENIED_LABEL),
        (ClientError::Unsupported, DENIED_LABEL),
    ] {
        let mut fixture = Fixture::new();
        fixture.state.writers.clear();
        control::acquire(&mut fixture.state, &fixture.jobs, fixture.target);
        let pending = acquisition(&mut fixture);
        fixture.finish(pending, Err(ControlError::Client(error)), true);
        assert_eq!(
            control::label(&fixture.state, fixture.target.address),
            label
        );
        assert!(!terminal_actions::deliver(
            &mut fixture.state,
            &fixture.jobs,
            Ok(NEW_INPUT.to_vec())
        ));
        assert_eq!(fixture.state.status, label);
        polling::renew_leases(&mut fixture.state, &fixture.jobs);
        fixture.no_jobs();
    }
}

#[test]
fn stale_same_connection_attempt_cannot_replace_pending_acquisition_or_current_writer() {
    let mut fixture = Fixture::new();
    fixture.state.writers.clear();
    control::acquire(&mut fixture.state, &fixture.jobs, fixture.target);
    let old = acquisition(&mut fixture);
    elapsed_retry(&mut fixture);
    assert!(!old.validity.as_ref().unwrap().is_current());
    polling::renew_leases(&mut fixture.state, &fixture.jobs);
    let current = acquisition(&mut fixture);
    assert_eq!(old.generation, current.generation);
    fixture.finish(
        old.clone(),
        Err(ControlError::Client(ClientError::Remote(
            PublicError::Unauthorized,
        ))),
        true,
    );
    assert_eq!(
        control::label(&fixture.state, fixture.target.address),
        RESTORING_LABEL
    );
    assert!(current.validity.as_ref().unwrap().is_current());
    let stale = fixture.grant;
    fixture.finish(old.clone(), Ok(Reply::Lease { grant: stale }), true);
    released(&mut fixture, stale);
    assert!(fixture.state.writers.is_empty());
    let grant = replacement(&fixture);
    fixture.finish(current.clone(), Ok(Reply::Lease { grant }), true);
    fixture.finish(current, Ok(Reply::Lease { grant }), true);
    assert_eq!(fixture.state.writers[&fixture.target.address].grant, grant);
    fixture.no_jobs();
    fixture.finish(old, Ok(Reply::Lease { grant: stale }), true);
    released(&mut fixture, stale);
    assert_eq!(fixture.state.writers[&fixture.target.address].grant, grant);
}
