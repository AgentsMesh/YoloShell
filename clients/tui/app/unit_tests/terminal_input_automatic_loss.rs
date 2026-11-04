use super::{query_support::Fixture, terminal_input_ordering::queued};
use crate::{events, network::Update, terminal_actions};
use std::time::Instant;
use yolo_hub_rpc_protocol::{Command, Reply};

pub(super) const PREFIX: &[u8] = b"printf '%s' 'complete";
pub(super) const SUFFIX: &[u8] = b"'\r";

pub(super) fn late_renewal(fixture: &mut Fixture) {
    let grant = fixture.grant;
    events::apply(
        &mut fixture.state,
        &fixture.jobs,
        Update::Reply {
            command: Command::Renew { grant },
            reply: Reply::Lease { grant },
        },
    );
    fixture.ready_size();
}

#[test]
fn automatic_expiry_or_resync_with_unconfirmed_input_cannot_resume_from_a_late_renewal() {
    for expire in [false, true] {
        let mut fixture = Fixture::new();
        let original = queued(&mut fixture, PREFIX);
        if expire {
            fixture
                .state
                .writers
                .get_mut(&fixture.target.address)
                .unwrap()
                .expires = Instant::now();
        } else {
            fixture.state.surfaces.remove(&fixture.target.address);
        }
        fixture.state.sync_query_authority();
        late_renewal(&mut fixture);
        assert!(!original.validity.as_ref().unwrap().is_current());
        assert!(
            fixture
                .state
                .grant(fixture.target.address, Instant::now())
                .is_none()
        );
        assert!(!terminal_actions::deliver(
            &mut fixture.state,
            &fixture.jobs,
            Ok(SUFFIX.to_vec())
        ));
        fixture.no_jobs();
    }
}

#[test]
fn a_fully_acknowledged_stream_can_resync_without_unnecessarily_losing_its_writer() {
    let mut fixture = Fixture::new();
    let request = queued(&mut fixture, PREFIX);
    fixture.finish(request, Ok(Reply::Input { duplicate: false }), true);
    fixture.state.mark_resync(fixture.target.address);
    fixture.state.sync_query_authority();
    fixture.state.clear_resync(fixture.target.address);
    late_renewal(&mut fixture);
    assert_eq!(
        fixture.state.writers[&fixture.target.address].grant,
        fixture.grant
    );
    let next = queued(&mut fixture, SUFFIX);
    assert!(next.validity.as_ref().unwrap().is_current());
}

#[test]
fn an_interrupted_background_writer_stays_blocked_until_an_actual_new_writer_is_confirmed() {
    let mut fixture = Fixture::new();
    let previous = queued(&mut fixture, PREFIX);
    fixture
        .state
        .writers
        .get_mut(&fixture.target.address)
        .unwrap()
        .expires = Instant::now();
    fixture.state.sync_query_authority();
    fixture.state.mode = crate::Mode::Inventory;
    fixture.state.sync_query_authority();
    late_renewal(&mut fixture);
    assert!(!fixture.state.writers.contains_key(&fixture.target.address));
    fixture.state.mode = crate::Mode::Terminal;
    fixture
        .state
        .inputs
        .resume_requested
        .insert(fixture.target.address);
    let replacement = fixture.replace_writer();
    let next = queued(&mut fixture, SUFFIX);
    fixture.finish(
        previous,
        Err(yolo_hub_client_channels::ControlError::Invalidated),
        true,
    );
    assert_eq!(
        fixture.state.writers[&fixture.target.address].grant,
        replacement
    );
    assert!(next.validity.as_ref().unwrap().is_current());
}
