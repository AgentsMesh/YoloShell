use super::{creation_support::*, finalization_support};
use crate::{
    events,
    network::{Job, Update},
};
use yolo_hub_client_channels::ControlEvent;
use yolo_hub_rpc_protocol::{Command, Reply};
use yolo_hub_terminal_protocol::{CreationQueryOutcome, CreationQueryResult};

#[test]
fn missing_create_callback_retires_view_intent_and_queries_without_replaying_the_operation() {
    let mut fixture = Fixture::new();
    let admitted = fixture.start();
    let original = create(&admitted);
    let expires = fixture.state.creations.pending[&original.fields().request_id].deadline;
    let before = fixture.state.tab_snapshot();
    crate::creation_recovery::flush(&mut fixture.state, &fixture.jobs, expires);
    assert!(fixture.state.creations.pending.is_empty());
    let Job::CreationQuery(query) = fixture.incoming.try_recv().unwrap() else {
        panic!()
    };
    let Command::QueryCreation { request } = &query.command else {
        panic!()
    };
    assert_eq!(request.creation(), original);
    assert_eq!(query.destination, admitted.destination);
    assert!(fixture.incoming.try_recv().is_err());
    let created = fixture.created();
    fixture.finish(admitted.clone(), created);
    events::apply(
        &mut fixture.state,
        &fixture.jobs,
        Update::NotSent {
            request: admitted.clone(),
        },
    );
    assert_eq!(fixture.state.tab_snapshot(), before);
    assert!(!fixture.state.creations.recovery.is_empty());
    fixture.assert_finalize(&admitted, created, false);
    let result = CreationQueryResult::new(
        original.fields().creation_token,
        CreationQueryOutcome::Ready { creation: created },
    )
    .unwrap();
    events::apply(
        &mut fixture.state,
        &fixture.jobs,
        Update::Control(ControlEvent::Finished {
            request: query,
            result: Ok(Reply::CreationQuery { query: result }),
            connected: true,
        }),
    );
    assert!(fixture.state.creations.recovery.is_empty());
    assert_eq!(fixture.state.tab_snapshot(), before);
    assert!(fixture.state.writers.is_empty());
    let finalized = finalization_support::take(&mut fixture);
    assert!(matches!(finalized.command, Command::Finalize { creation, .. } if creation == created));
}
