use super::constants::*;
use uuid::Uuid;
use yolo_hub_agent_protocol::{
    Activity, AgentWorkingDirectory, Lifecycle, ObservationCursor, ObservedProvider,
};
use yolo_hub_connector_protocol::*;
use yolo_hub_identity::*;

pub struct Fixture {
    pub hub: HubId,
    pub enrollment: Enrollment,
    pub activation: Activation,
    pub publication: Publication,
    pub ack: PublicationAck,
}

impl Fixture {
    pub fn new() -> Self {
        let hub = HubId::new();
        let scope = ConnectorScope {
            terminal: RuntimeTarget {
                address: TerminalAddress {
                    hub_id: hub,
                    terminal_id: TerminalId::new(),
                },
                runtime_epoch: RuntimeEpoch::new(),
            },
            provider: ObservedProvider::new(
                PROVIDER_KIND.into(),
                Some(PROVIDER_VERSION.into()),
                Some(PROVIDER_REFERENCE.into()),
            )
            .unwrap(),
        };
        let enrollment = Enrollment {
            connector_id: ConnectorId::new(),
            agent: AgentAddress {
                hub_id: hub,
                agent_id: AgentId::new(),
            },
            scope,
            credential: ConnectorCredential::generate(),
            revision: FIRST_CONNECTOR_REVISION,
        };
        let activation = Activation {
            connector_id: enrollment.connector_id,
            generation: ConnectorGeneration::new(),
            agent: AgentTarget {
                address: enrollment.agent,
                execution_id: ExecutionId::new(),
            },
            terminal: enrollment.scope.terminal,
            next_sequence: FIRST_PUBLICATION_SEQUENCE,
        };
        let publication = Publication {
            connector_id: activation.connector_id,
            generation: activation.generation,
            agent: activation.agent,
            terminal: activation.terminal,
            sequence: FIRST_PUBLICATION_SEQUENCE,
            state: ReportedState {
                lifecycle: Lifecycle::Running {},
                activity: Activity::Working,
                working_directory: Some(AgentWorkingDirectory::new(DIRECTORY.into()).unwrap()),
            },
        };
        let ack = PublicationAck {
            connector_id: publication.connector_id,
            generation: publication.generation,
            sequence: publication.sequence,
            cursor: ObservationCursor::new(publication.agent, Uuid::new_v4(), OBSERVATION_SEQUENCE)
                .unwrap(),
            duplicate: false,
        };
        Self {
            hub,
            enrollment,
            activation,
            publication,
            ack,
        }
    }

    pub fn summary(&self, status: EnrollmentStatus) -> EnrollmentSummary {
        EnrollmentSummary {
            connector_id: self.enrollment.connector_id,
            agent: self.enrollment.agent,
            scope: self.enrollment.scope.clone(),
            revision: self.enrollment.revision,
            status,
        }
    }

    pub fn samples(&self) -> Vec<(Request, Reply)> {
        let mut revoked = self.summary(EnrollmentStatus::Revoked);
        revoked.revision += REVISION_INCREMENT;
        [
            (Command::Hello {}, Reply::Hello { hub: self.hub }),
            (
                Command::List {},
                Reply::Enrollments {
                    enrollments: vec![self.summary(EnrollmentStatus::Enabled)],
                },
            ),
            (
                Command::Register {
                    scope: self.enrollment.scope.clone(),
                },
                Reply::Enrolled {
                    enrollment: self.enrollment.clone(),
                },
            ),
            (
                Command::Activate {
                    connector_id: self.enrollment.connector_id,
                    credential: self.enrollment.credential,
                    instance_id: ConnectorInstanceId::new(),
                },
                Reply::Activated {
                    activation: self.activation,
                },
            ),
            (
                Command::Publish {
                    publication: self.publication.clone(),
                },
                Reply::Published { ack: self.ack },
            ),
            (
                Command::Revoke {
                    connector_id: self.enrollment.connector_id,
                    expected_revision: self.enrollment.revision,
                },
                Reply::Revoked {
                    enrollment: revoked,
                },
            ),
        ]
        .into_iter()
        .map(|(command, reply)| (Request::new(self.hub, command).unwrap(), reply))
        .collect()
    }
}
