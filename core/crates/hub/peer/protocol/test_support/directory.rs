use super::Fixture;
use uuid::Uuid;
use yolo_hub_agent_protocol::*;
use yolo_hub_identity::{AgentAddress, AgentId, AgentTarget, ExecutionId};
use yolo_hub_peer_protocol::{Purpose, Signed};

pub const AGENT_DIRECTORY: &str = "/remote/origin/项目 'space' $value `literal`";
pub const REPLACEMENT_DIRECTORY: &str = "/relay/substituted";
pub const PROVIDER_KIND: &str = "example.agent";
pub const OBSERVED_UNIX_MS: u64 = 1;

pub fn fixture(directory: Option<&str>) -> Fixture {
    let mut fixture = Fixture::new();
    let terminal = fixture.export.catalogue.payload.terminals[0]
        .fields()
        .runtime
        .unwrap()
        .target;
    let target = AgentTarget {
        address: AgentAddress {
            hub_id: fixture.origin.identity.hub(),
            agent_id: AgentId::new(),
        },
        execution_id: ExecutionId::new(),
    };
    let snapshot = AgentSnapshot::new(AgentSnapshotFields {
        schema_version: AGENT_PROTOCOL_SCHEMA_VERSION,
        cursor: ObservationCursor::new(target, Uuid::new_v4(), FIRST_OBSERVATION_SEQUENCE).unwrap(),
        terminal,
        provider: ObservedProvider::new(PROVIDER_KIND.into(), None, None).unwrap(),
        working_directory: directory.map(|path| AgentWorkingDirectory::new(path.into()).unwrap()),
        lifecycle: Lifecycle::Running {},
        activity: Activity::Working,
        reachability: Reachability::Reachable,
        freshness: Freshness::Current,
        source: ObservationSource::ProviderHook,
        observed_unix_ms: OBSERVED_UNIX_MS,
        recovery: RecoveryStatus::Unavailable,
        capabilities: AgentCapabilities::UNSUPPORTED,
        interaction: None,
    })
    .unwrap();
    let mut catalogue = fixture.export.catalogue.payload.clone();
    catalogue.agents = vec![snapshot];
    fixture.export.catalogue = Signed::new(catalogue, Purpose::Catalogue, &fixture.origin).unwrap();
    fixture
}
