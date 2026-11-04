use super::fixture::*;
use crate::role::Role;
use yolo_hub_connector_protocol::{Command, ConnectorError};
use yolo_hub_identity::{AgentId, ConnectorGeneration, ConnectorId, ExecutionId, RuntimeEpoch};

#[test]
fn anonymous_admin_and_activated_publisher_have_separate_command_sets() {
    let fixture = Fixture::new();
    let activation = fixture.activation();
    let admin = Role::default();
    let publisher = Role {
        greeted: true,
        activation: Some(activation),
    };
    for command in commands(activation) {
        if matches!(command, Command::Publish { .. }) {
            assert_eq!(admin.authorize(&command), Err(ConnectorError::Unauthorized));
        } else {
            assert_eq!(admin.authorize(&command), Ok(()));
        }
        if matches!(command, Command::Hello {} | Command::Publish { .. }) {
            assert_eq!(publisher.authorize(&command), Ok(()));
        } else {
            assert_eq!(
                publisher.authorize(&command),
                Err(ConnectorError::Unauthorized)
            );
        }
    }
}

#[test]
fn publisher_role_requires_connector_generation_agent_execution_and_terminal() {
    let fixture = Fixture::new();
    let active = fixture.activation();
    let role = Role {
        greeted: true,
        activation: Some(active),
    };
    let mutations: &[fn(&mut yolo_hub_connector_protocol::Publication)] = &[
        |p| p.connector_id = ConnectorId::new(),
        |p| p.generation = ConnectorGeneration::new(),
        |p| p.agent.address.agent_id = AgentId::new(),
        |p| p.agent.execution_id = ExecutionId::new(),
        |p| p.terminal.runtime_epoch = RuntimeEpoch::new(),
        |p| p.terminal.address.terminal_id = yolo_hub_identity::TerminalId::new(),
    ];
    for mutate in mutations {
        let mut forged = publication(active);
        mutate(&mut forged);
        assert_eq!(
            role.authorize(&Command::Publish {
                publication: forged
            }),
            Err(ConnectorError::StaleGeneration)
        );
    }
    assert_eq!(
        role.authorize(&Command::Publish {
            publication: publication(active)
        }),
        Ok(())
    );
}
