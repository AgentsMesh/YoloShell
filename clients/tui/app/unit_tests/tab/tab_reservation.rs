use super::creation_support::Fixture;
use crate::{network::Job, tabs, terminal_actions};
use crossterm::event::KeyCode;
use yolo_hub_rpc_protocol::Command;

const SINGLE: usize = 1;

fn prefix(fixture: &mut Fixture, key: char) {
    terminal_actions::prefix(
        &mut fixture.state,
        &fixture.jobs,
        fixture.origin,
        KeyCode::Char(key),
    );
}

#[test]
fn nested_split_and_closed_pending_destination_can_admit_a_new_shell() {
    let mut fixture = Fixture::new();
    fixture.start();
    prefix(&mut fixture, tabs::SPLIT_RIGHT_KEY);
    fixture.incoming.try_recv().unwrap();
    prefix(&mut fixture, tabs::SPLIT_DOWN_KEY);
    fixture.incoming.try_recv().unwrap();
    prefix(&mut fixture, tabs::CLOSE_TAB_KEY);
    assert!(
        fixture
            .state
            .creations
            .pending
            .values()
            .all(|pending| !pending.binding_active)
    );
    prefix(&mut fixture, tabs::SPLIT_RIGHT_KEY);
    assert_eq!(fixture.state.tab_layout().panes().len(), SINGLE);
    assert!(
        fixture
            .state
            .creations
            .pending
            .values()
            .any(|pending| pending.binding_active)
    );
    assert!(matches!(
        fixture.incoming.try_recv().unwrap(),
        Job::Call {
            command: Command::Create { .. },
            ..
        }
    ));
    assert!(fixture.incoming.try_recv().is_err());
}
