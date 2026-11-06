use super::{
    query_support::Fixture,
    terminal_input_navigation_support::{PREFIX, unknown},
    terminal_input_ordering::queued,
};
use crate::{Mode, network::Job, terminal_input_authority};
use yolo_hub_terminal_protocol::{LeaseMode, TerminalSummary};

#[test]
fn catalog_epoch_change_before_writer_grant_does_not_inherit_old_view_interruption() {
    let mut fixture = Fixture::new();
    let original = queued(&mut fixture, PREFIX);
    assert_eq!(fixture.state.mode, Mode::Terminal);
    let pane = fixture.state.tab_layout().focused();
    let mut terminal = fixture.state.terminals[&fixture.target.address]
        .fields()
        .clone();
    let runtime = terminal.runtime.as_mut().unwrap();
    runtime.target.runtime_epoch = yolo_hub_identity::RuntimeEpoch::new();
    let target = runtime.target;

    // Real catalogue order: replacement is observed before any writer reply.
    fixture
        .state
        .replace_terminals(vec![TerminalSummary::new(terminal).unwrap()]);

    assert_eq!(fixture.state.mode, Mode::Terminal);
    assert_eq!(fixture.state.tab_layout().focused(), pane);
    assert!(fixture.state.writers.is_empty());
    assert!(fixture.state.inputs.scope.is_none());
    assert!(!original.validity.as_ref().unwrap().is_current());
    assert!(!terminal_input_authority::blocked(
        &fixture.state,
        target.address
    ));
    crate::polling::acquire(&mut fixture.state, &fixture.jobs, false);
    let Job::Acquire { request, .. } = fixture.incoming.try_recv().unwrap() else {
        panic!();
    };
    assert_eq!(request.target, target);
    assert_eq!(request.mode, LeaseMode::Acquire {});

    fixture.finish(original, unknown(), true);

    assert!(!terminal_input_authority::blocked(
        &fixture.state,
        target.address
    ));
    assert!(fixture.state.inputs.pending.is_empty());
    fixture.no_jobs();
}
