use crate::{
    commands::*,
    fixtures::{constants::*, *},
};
use uuid::Uuid;
use yolo_hub_rpc_protocol::{
    Command, HUB_PROTOCOL_VERSION, Method, Request, RequestFields, SUPPORTED_METHODS,
};
use yolo_hub_terminal_protocol::PublicError;

#[test]
fn every_command_has_stable_method_origin_and_mutation_semantics() {
    assert_eq!(samples().len(), SUPPORTED_METHODS.len());
    for method in SUPPORTED_METHODS {
        let (command, _) = samples()
            .into_iter()
            .find(|(command, _)| command.method() == *method)
            .unwrap();
        assert_eq!(command.method(), *method);
        assert_eq!(
            command.mutates(),
            !matches!(
                method,
                Method::Hello
                    | Method::List
                    | Method::Get
                    | Method::QueryCreation
                    | Method::Attach
                    | Method::ReadTerminalText
                    | Method::ReadTerminalHistory
                    | Method::ObserveLease
                    | Method::ListAgents
                    | Method::WaitAgents
                    | Method::GetAgent
                    | Method::GetCurrentAgent
                    | Method::ListHubs
                    | Method::ListProviders
                    | Method::GetPlatformSnapshot
                    | Method::ListPlatformEvents
                    | Method::ExportAudit
                    | Method::SubscribeEvents
                    | Method::ListFileRoots
                    | Method::ReadFile
                    | Method::ListLayouts
                    | Method::GetLayout
                    | Method::ResolveConversation
                    | Method::ReadConversation
                    | Method::WatchConversation
                    | Method::GetWorkspace
                    | Method::ListWorkspaces
                    | Method::GetAgentRun
                    | Method::ListAgentRuns
                    | Method::ListWorktrees
            )
        );
        let catalogue_affecting = matches!(
            method,
            Method::Create
                | Method::TerminalRun
                | Method::AgentStart
                | Method::AgentStop
                | Method::Finalize
                | Method::Terminal
                | Method::CleanupTerminate
                | Method::CleanupArchive
        );
        assert_eq!(command.affects_catalogue(), catalogue_affecting);
        assert_eq!(command.validate_origin(hub(HUB_ID)), Ok(()));
        let ownerless = matches!(
            method,
            Method::Hello
                | Method::List
                | Method::Create
                | Method::TerminalRun
                | Method::AgentStart
                | Method::CreateWorkspace
                | Method::ListWorkspaces
                | Method::ListAgentRuns
                | Method::QueryCreation
                | Method::ListAgents
                | Method::WaitAgents
                | Method::ListHubs
                | Method::ListProviders
                | Method::GetPlatformSnapshot
                | Method::ListPlatformEvents
                | Method::ExportAudit
                | Method::SubscribeEvents
                | Method::ListFileRoots
                | Method::ListLayouts
        );
        assert_eq!(
            command.validate_origin(hub(SECOND_HUB_ID)),
            if ownerless {
                Ok(())
            } else {
                Err(PublicError::OriginMismatch)
            }
        );
        let correlated = matches!(
            method,
            Method::Create
                | Method::Lease
                | Method::Input
                | Method::Resize
                | Method::Terminal
                | Method::AgentControl
                | Method::UpdateLayout
                | Method::UpdateAgentRun
                | Method::CreateWorktree
                | Method::RemoveWorktree
        );
        assert_eq!(command.request_id(), correlated.then(|| id(REQUEST_ID)));
        let request = request(command);
        assert!(!request.fields().request_id.is_nil());
        assert_eq!(request.fields().version, HUB_PROTOCOL_VERSION);
        let fields: RequestFields = request.clone().into();
        assert_eq!(Request::try_from(fields).unwrap(), request);
    }
}

#[test]
fn invalid_envelope_never_changes_body_request_identity() {
    let original = request(Command::Input { request: input() });
    let cases = [
        (
            HUB_PROTOCOL_VERSION + SCHEMA_STEP,
            id(REQUEST_ID),
            PublicError::UnsupportedSchema,
        ),
        (
            HUB_PROTOCOL_VERSION,
            Uuid::nil(),
            PublicError::InvalidRequest,
        ),
        (
            HUB_PROTOCOL_VERSION,
            id(SECOND_REQUEST_ID),
            PublicError::InvalidRequest,
        ),
    ];
    for (version, request_id, expected) in cases {
        let mut fields = original.fields().clone();
        fields.version = version;
        fields.request_id = request_id;
        assert_eq!(Request::try_from(fields), Err(expected));
    }
    let hello = request(Command::Hello {});
    assert_eq!(Request::try_from(hello.fields().clone()), Ok(hello));
}

#[test]
fn raw_command_fields_are_validated_before_request_construction() {
    let mut invalid_lease = lease();
    invalid_lease.principal.client_id = Uuid::nil();
    let mut invalid_resize = resize();
    invalid_resize.request_id = Uuid::nil();
    let mut invalid_session = session_command();
    invalid_session.request_id = Uuid::nil();
    let cases = [
        Command::Finalize {
            creation_token: Uuid::nil(),
            creation: creation(),
        },
        Command::Lease {
            request: invalid_lease,
        },
        Command::Resize {
            request: invalid_resize,
        },
        Command::Terminal {
            command: invalid_session,
        },
    ];
    for command in cases {
        assert_eq!(
            command.validate_origin(hub(HUB_ID)),
            Err(PublicError::InvalidRequest)
        );
        assert_eq!(
            Request::new(hub(HUB_ID), command),
            Err(PublicError::InvalidRequest)
        );
    }
}

#[test]
fn ownerless_catalog_cursor_can_address_a_transit_origin() {
    let command = Command::List {
        include_archived: false,
        after: Some(address(SECOND_HUB_ID, SESSION_ID)),
    };
    assert!(Request::new(hub(HUB_ID), command).is_ok());
}
