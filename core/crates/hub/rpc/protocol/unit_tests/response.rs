use crate::{
    commands::*,
    fixtures::{constants::*, *},
};
use yolo_hub_rpc_protocol::{
    Command, HUB_PROTOCOL_VERSION, Hello, Method, Reply, Response, SUPPORTED_METHODS,
};
use yolo_hub_terminal_protocol::PublicError;

#[test]
fn response_envelope_is_fenced_by_version_origin_request_id_and_method() {
    let request = request(Command::Get {
        address: target().address,
    });
    let valid = Response::new(
        &request,
        hub(HUB_ID),
        Reply::Terminal {
            terminal: summary(HUB_ID, SESSION_ID),
        },
    );
    assert_eq!(valid.validate_for(&request), Ok(()));
    let mut invalid = valid.clone();
    invalid.version += SCHEMA_STEP;
    assert_eq!(
        invalid.validate_for(&request),
        Err(PublicError::UnsupportedSchema)
    );
    invalid = valid.clone();
    invalid.origin = hub(SECOND_HUB_ID);
    assert_eq!(
        invalid.validate_for(&request),
        Err(PublicError::OriginMismatch)
    );
    invalid = valid.clone();
    invalid.request_id = id(SECOND_REQUEST_ID);
    assert_eq!(
        invalid.validate_for(&request),
        Err(PublicError::InvalidRequest)
    );
    invalid = valid;
    invalid.method = Method::List;
    assert_eq!(
        invalid.validate_for(&request),
        Err(PublicError::InvalidRequest)
    );
}

#[test]
fn every_reply_family_matches_only_its_declared_methods() {
    for (command, own_reply) in samples() {
        let request = request(command);
        for (_, candidate) in samples() {
            let mut compatible =
                std::mem::discriminant(&own_reply) == std::mem::discriminant(&candidate);
            if request.fields().command.method() == Method::CleanupArchive {
                compatible &= matches!(&candidate, Reply::Terminal { terminal: session } if session.fields().archived);
            }
            if matches!(
                &candidate,
                Reply::Workspace { .. } | Reply::Workspaces { .. } | Reply::AgentRun { .. }
            ) {
                compatible &= yolo_hub_rpc_protocol::reply_matches_target(
                    &candidate,
                    &request.fields().command,
                );
            }
            let response = Response::new(&request, hub(HUB_ID), candidate);
            assert_eq!(response.validate_for(&request).is_ok(), compatible);
        }
        let error = Response::new(
            &request,
            hub(HUB_ID),
            Reply::Error {
                code: PublicError::Unauthorized,
            },
        );
        assert_eq!(error.validate_for(&request), Ok(()));
    }
}

#[test]
fn hello_identity_schema_and_method_set_are_validated() {
    let request = request(Command::Hello {});
    let valid = Hello {
        hub_id: hub(HUB_ID),
        version: HUB_PROTOCOL_VERSION,
        methods: SUPPORTED_METHODS.to_vec(),
    };
    let mut wrong_origin = valid.clone();
    wrong_origin.hub_id = hub(SECOND_HUB_ID);
    let mut wrong_version = valid.clone();
    wrong_version.version += SCHEMA_STEP;
    let mut too_many = valid.clone();
    too_many.methods.push(Method::Hello);
    let mut duplicate = valid.clone();
    duplicate.methods = vec![Method::Hello, Method::Hello];
    for hello in [wrong_origin, wrong_version, too_many, duplicate] {
        assert_eq!(
            Response::new(&request, hub(HUB_ID), Reply::Hello { hello }).validate_for(&request),
            Err(PublicError::InvalidPayload)
        );
    }
    let empty = Hello {
        methods: Vec::new(),
        ..valid
    };
    assert_eq!(
        Response::new(&request, hub(HUB_ID), Reply::Hello { hello: empty }).validate_for(&request),
        Ok(())
    );
}
