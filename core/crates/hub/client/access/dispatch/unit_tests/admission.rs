use super::model::*;
use crate::admission::{advertised, authorize, visible};
use yolo_hub_routing::Right;
use yolo_hub_rpc_protocol::{Command, Method};
use yolo_hub_terminal_protocol::{CreationQueryRequest, PublicError};

#[test]
fn aggregate_queries_cannot_bypass_entry_scope_by_targeting_an_origin() {
    let (_registry, admission) = authority(&[Right::Observe]);
    for command in [
        Command::Hello {},
        Command::List {
            include_archived: false,
            after: None,
        },
        Command::ListAgents { after: None },
        Command::ListHubs { after: None },
    ] {
        assert_eq!(
            authorize(&admission, &request(ENTRY, command.clone())),
            Ok(())
        );
        assert_eq!(
            authorize(&admission, &request(ORIGIN, command)),
            Err(PublicError::Unauthorized)
        );
    }
    assert!(visible(&admission, hub(ORIGIN)));
    assert!(!visible(&admission, hub(HIDDEN)));
}

#[test]
fn observe_cannot_create_query_or_gain_terminal_authority_and_local_management_is_denied() {
    let (registry, admission) = authority(&[Right::Observe]);
    for command in [
        Command::Create { request: create() },
        Command::QueryCreation {
            request: CreationQueryRequest::new(create()),
        },
        Command::GetBuild {},
        Command::GetConfiguration {},
        Command::GetSshProfiles {},
        Command::ListFileRoots { after: None },
    ] {
        assert_eq!(
            authorize(&admission, &request(ORIGIN, command)),
            Err(PublicError::Unauthorized)
        );
    }
    for command in writer_commands(CLIENT) {
        assert_eq!(
            authorize(&admission, &request(ORIGIN, command)),
            Err(PublicError::Unauthorized)
        );
    }
    assert!(advertised(&admission, Method::Hello));
    assert!(advertised(&admission, Method::Get));
    assert!(!advertised(&admission, Method::Input));
    assert!(!advertised(&admission, Method::Configure));
    registry.close();
    assert_eq!(
        authorize(&admission, &request(ENTRY, Command::Hello {})),
        Err(PublicError::Unauthorized)
    );
}

#[test]
fn every_terminal_mutation_requires_the_authenticated_client_principal() {
    let (_registry, admission) = authority(&[Right::TerminalWrite]);
    for command in writer_commands(CLIENT) {
        assert_eq!(authorize(&admission, &request(ORIGIN, command)), Ok(()));
    }
    for command in writer_commands(OTHER_CLIENT) {
        assert_eq!(
            authorize(&admission, &request(ORIGIN, command)),
            Err(PublicError::Unauthorized)
        );
    }
    assert!(!visible(&admission, hub(ORIGIN)));
    assert!(!advertised(&admission, Method::AgentControl));
}

#[test]
fn lifecycle_authorizes_result_lookup_without_implying_observation_or_writer_rights() {
    let (_registry, admission) = authority(&[Right::Lifecycle]);
    assert_eq!(
        authorize(
            &admission,
            &request(
                ORIGIN,
                Command::QueryCreation {
                    request: CreationQueryRequest::new(create())
                }
            )
        ),
        Ok(())
    );
    assert_eq!(
        authorize(
            &admission,
            &request(HIDDEN, Command::Create { request: create() })
        ),
        Err(PublicError::Unauthorized)
    );
    assert!(!advertised(&admission, Method::Get));
    assert!(advertised(&admission, Method::QueryCreation));
}
