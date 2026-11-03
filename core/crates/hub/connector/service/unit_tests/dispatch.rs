use super::fixture::*;
use crate::{dispatch::dispatch, role::Role};
use yolo_hub_connector_protocol::{Command, ConnectorError, Reply};

#[tokio::test]
async fn anonymous_publication_and_publisher_admin_commands_fail_before_contacting_host() {
    let fixture = Fixture::new();
    let activation = fixture.activation();
    let mut role = Role::default();
    assert_eq!(
        dispatch(
            &fixture.backend,
            &mut role,
            &fixture.request(Command::Publish {
                publication: publication(activation)
            })
        )
        .await,
        Err(ConnectorError::Unauthorized)
    );
    role.activation = Some(activation);
    for command in commands(activation) {
        if matches!(command, Command::Hello {} | Command::Publish { .. }) {
            continue;
        }
        assert_eq!(
            dispatch(&fixture.backend, &mut role, &fixture.request(command)).await,
            Err(ConnectorError::Unauthorized)
        );
    }
    assert_eq!(role.activation, Some(activation));
}

#[tokio::test]
async fn hello_and_empty_inventory_use_local_registry_without_host_process() {
    let fixture = Fixture::new();
    let mut role = Role::default();
    assert_eq!(
        dispatch(
            &fixture.backend,
            &mut role,
            &fixture.request(Command::Hello {})
        )
        .await
        .unwrap(),
        Reply::Hello {
            hub: fixture.state.hub_id()
        }
    );
    assert_eq!(
        dispatch(
            &fixture.backend,
            &mut role,
            &fixture.request(Command::List {})
        )
        .await
        .unwrap(),
        Reply::Enrollments {
            enrollments: vec![]
        }
    );
    assert!(role.activation.is_none());
}

#[tokio::test]
async fn missing_activation_or_revocation_is_not_a_successful_mutation() {
    let fixture = Fixture::new();
    let active = fixture.activation();
    let mut role = Role::default();
    for command in commands(active) {
        if !matches!(command, Command::Activate { .. } | Command::Revoke { .. }) {
            continue;
        }
        assert_eq!(
            dispatch(&fixture.backend, &mut role, &fixture.request(command)).await,
            Err(ConnectorError::NotFound)
        );
        assert!(role.activation.is_none());
    }
}
