use super::{
    creation_support::{Fixture, create},
    finalization_support::*,
};
use crate::creation_actions;
use yolo_hub_rpc_protocol::Reply;
use yolo_hub_terminal_protocol::CreateShellRequest;

#[test]
fn retained_finalizations_share_the_creation_capacity_until_acknowledged() {
    let mut fixture = Fixture::new();
    let limit = crate::creation_constants::MAX_PENDING_CREATIONS;
    let (first_create, _, first_finalize) = confirmed(&mut fixture);
    for _ in 1..limit {
        confirmed(&mut fixture);
    }
    assert!(fixture.state.creations.pending.is_empty());
    assert_eq!(fixture.state.creations.finalizing.len(), limit);
    let mut fields = create(&first_create).fields().clone();
    fields.request_id = uuid::Uuid::new_v4();
    fields.creation_token = uuid::Uuid::new_v4();
    let next = CreateShellRequest::new(fields).unwrap();
    assert!(!creation_actions::enqueue(
        &mut fixture.state,
        &fixture.jobs,
        fixture.origin,
        next.clone()
    ));
    no_commands(&mut fixture);
    finish(&mut fixture, first_finalize, Ok(Reply::Completed {}));
    no_commands(&mut fixture);
    assert!(creation_actions::enqueue(
        &mut fixture.state,
        &fixture.jobs,
        fixture.origin,
        next
    ));
    assert_eq!(fixture.state.creations.len(), limit);
}

#[test]
fn new_creation_cannot_reuse_a_retained_request_id_or_creation_token() {
    for reused_request in [true, false] {
        let mut fixture = Fixture::new();
        let (admitted, _, _) = confirmed(&mut fixture);
        let mut fields = create(&admitted).fields().clone();
        if reused_request {
            fields.creation_token = uuid::Uuid::new_v4();
        } else {
            fields.request_id = uuid::Uuid::new_v4();
        }
        assert!(!creation_actions::enqueue(
            &mut fixture.state,
            &fixture.jobs,
            fixture.origin,
            CreateShellRequest::new(fields).unwrap(),
        ));
        assert_eq!(fixture.state.creations.finalizing.len(), 1);
        assert!(fixture.state.creations.pending.is_empty());
        no_commands(&mut fixture);
    }
}
