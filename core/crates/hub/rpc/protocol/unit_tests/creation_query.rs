use crate::{
    creation_query_support::*,
    fixtures::{self, constants::*},
};
use std::time::Duration;
use yolo_hub_rpc_protocol::{
    Command, HUB_PROTOCOL_VERSION, Method, Reply, Request, Response, SUPPORTED_METHODS,
};
use yolo_hub_terminal_protocol::{CreationQueryOutcome, PublicError};

#[test]
fn creation_queries_are_read_only_routable_and_have_independent_transport_correlation() {
    let command = command();
    assert_eq!(command.method(), Method::QueryCreation);
    assert!(!command.mutates());
    assert!(!command.local_only());
    assert!(command.request_id().is_none());
    assert_eq!(command.operation_timeout(Duration::ZERO), Duration::ZERO);
    assert!(SUPPORTED_METHODS.contains(&Method::QueryCreation));
    for origin in [HUB_ID, SECOND_HUB_ID] {
        assert_eq!(command.validate_origin(fixtures::hub(origin)), Ok(()));
    }
    let first = Request::new(fixtures::hub(HUB_ID), command.clone()).unwrap();
    let next = Request::new(fixtures::hub(HUB_ID), command.clone()).unwrap();
    assert_ne!(first.fields().request_id, next.fields().request_id);
    assert_ne!(
        first.fields().request_id,
        fixtures::create().fields().request_id
    );
    assert_eq!(first.fields().command, next.fields().command);
    let debug = format!("{command:?}");
    assert!(debug.contains(REDACTED));
    assert!(!debug.contains(DIRECTORY));
    assert!(!debug.contains(&token().to_string()));
}

#[test]
fn query_replies_require_the_original_token_and_exact_addressed_origin() {
    let request = Request::new(fixtures::hub(HUB_ID), command()).unwrap();
    for outcome in outcomes(HUB_ID) {
        let correct = Response::new(
            &request,
            fixtures::hub(HUB_ID),
            reply(outcome.clone(), token()),
        );
        assert_eq!(correct.validate_for(&request), Ok(()));
        let different_token = Response::new(
            &request,
            fixtures::hub(HUB_ID),
            reply(outcome, fixtures::id(SECOND_REQUEST_ID)),
        );
        assert_eq!(
            different_token.validate_for(&request),
            Err(PublicError::InvalidPayload)
        );
    }
    for outcome in outcomes(SECOND_HUB_ID) {
        if matches!(
            outcome,
            CreationQueryOutcome::Missing {}
                | CreationQueryOutcome::Pending {}
                | CreationQueryOutcome::Unknown {}
        ) {
            continue;
        }
        let response = Response::new(&request, fixtures::hub(HUB_ID), reply(outcome, token()));
        assert_eq!(
            response.validate_for(&request),
            Err(PublicError::InvalidPayload)
        );
    }
    let correct = Response::new(
        &request,
        fixtures::hub(HUB_ID),
        reply(CreationQueryOutcome::Missing {}, token()),
    );
    let mut foreign_envelope = correct.clone();
    foreign_envelope.origin = fixtures::hub(SECOND_HUB_ID);
    assert_eq!(
        foreign_envelope.validate_for(&request),
        Err(PublicError::OriginMismatch)
    );
    let mut old_version = correct;
    old_version.version = PREVIOUS_PROTOCOL_VERSION;
    assert!(HUB_PROTOCOL_VERSION > PREVIOUS_PROTOCOL_VERSION);
    assert_eq!(
        old_version.validate_for(&request),
        Err(PublicError::UnsupportedSchema)
    );
}

#[test]
fn create_and_query_results_cannot_be_substituted_or_used_as_mutation_acknowledgements() {
    let query = Request::new(fixtures::hub(HUB_ID), command()).unwrap();
    for reply in [
        Reply::Created {
            creation: fixtures::creation(),
        },
        Reply::Completed {},
    ] {
        assert_eq!(
            Response::new(&query, fixtures::hub(HUB_ID), reply).validate_for(&query),
            Err(PublicError::InvalidPayload)
        );
    }
    let create = Request::new(
        fixtures::hub(HUB_ID),
        Command::Create {
            request: fixtures::create(),
        },
    )
    .unwrap();
    assert_eq!(
        Response::new(
            &create,
            fixtures::hub(HUB_ID),
            reply(
                CreationQueryOutcome::Ready {
                    creation: fixtures::creation()
                },
                token()
            )
        )
        .validate_for(&create),
        Err(PublicError::InvalidPayload)
    );
    let mut fields = query.fields().clone();
    fields.version = PREVIOUS_PROTOCOL_VERSION;
    assert!(Request::try_from(fields).is_err());
}
