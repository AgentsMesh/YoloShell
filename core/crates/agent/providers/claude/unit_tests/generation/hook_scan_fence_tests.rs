use super::super::{GenerationEvent, SessionStartSource};
use super::support::*;
use crate::agent::{
    AgentBinding, AgentBindingStore, AgentKind, AgentRecoveryAvailability, AgentState,
};

const ABSOLUTE_CLAUDE: &str = "/managed/agent-bin/claude";
const ABSOLUTE_NODE: &str = "/managed/node-bin/node";
const FOREIGN_SUFFIX: &str = "-foreign";
const IDENTITY_INCREMENT: u64 = 1;

#[test]
fn foreign_unmanaged_scans_preserve_canonical_binding_and_recovery_chain() {
    for mutate in foreign_mutations() {
        let (fixture, authority, store, canonical) = published();
        let mut detected = unmanaged_scan(&canonical);
        mutate(&mut detected);

        let retained = store
            .reconcile_detected_in_namespace(&fixture.lease, &detected, fixture.runtime())
            .unwrap();
        assert_eq!(retained, canonical);
        assert_eq!(raw(&fixture, &store), canonical);

        let restored = store
            .reconcile_detected_in_namespace(
                &fixture.lease,
                &unmanaged_scan(&canonical),
                fixture.runtime(),
            )
            .unwrap();
        assert!(authority.matches_binding(&restored));
        assert_eq!(raw(&fixture, &store), canonical);
    }
}

#[test]
fn node_basename_cannot_use_binding_only_repair_path() {
    let (fixture, authority, store, canonical) = published();
    let mut legacy = canonical.clone();
    legacy.state = AgentState::Unknown;
    fixture
        .lease
        .write_json(&fixture.binding_path(), &legacy)
        .unwrap();
    let mut detected = unmanaged_scan(&canonical);
    detected.executable = ABSOLUTE_NODE.to_owned();

    let retained = store
        .reconcile_detected_in_namespace(&fixture.lease, &detected, fixture.runtime())
        .unwrap();
    assert_eq!(retained, legacy);
    assert!(!authority.matches_binding(&retained));

    let restored = store
        .reconcile_detected_in_namespace(
            &fixture.lease,
            &unmanaged_scan(&canonical),
            fixture.runtime(),
        )
        .unwrap();
    assert!(authority.matches_binding(&restored));
}

fn published() -> (
    Fixture,
    super::super::ClaudeGenerationAuthority,
    AgentBindingStore,
    AgentBinding,
) {
    let fixture = Fixture::new();
    fixture
        .store
        .begin_in_namespace(&fixture.lease, &fixture.begin)
        .unwrap();
    let start = fixture.start(&fixture.begin.provider_session_id, SessionStartSource::Fresh);
    assert!(matches!(start, GenerationEvent::SessionStart { .. }));
    let authority = fixture
        .store
        .apply_in_namespace(&fixture.lease, fixture.authentication(), &start)
        .unwrap()
        .0;
    let canonical = binding(&authority);
    let store = AgentBindingStore::new(fixture.binding_path());
    store
        .save_authoritative_in_namespace(&fixture.lease, &canonical, &authority, fixture.runtime())
        .unwrap();
    (fixture, authority, store, canonical)
}

fn unmanaged_scan(canonical: &AgentBinding) -> AgentBinding {
    let mut detected = canonical.clone();
    detected.executable = ABSOLUTE_CLAUDE.to_owned();
    detected.state = AgentState::Unknown;
    detected.recovery_availability = AgentRecoveryAvailability::UnmanagedLaunch;
    detected
}

fn raw(fixture: &Fixture, store: &AgentBindingStore) -> AgentBinding {
    store
        .read_raw_in_namespace(&fixture.lease)
        .unwrap()
        .unwrap()
}

type Mutation = fn(&mut AgentBinding);

fn foreign_mutations() -> Vec<Mutation> {
    vec![
        |value| value.schema_version += IDENTITY_INCREMENT as u32,
        |value| value.terminal_id = uuid::Uuid::new_v4(),
        |value| value.kind = AgentKind::Codex,
        |value| value.provider_session_id = session_id(),
        |value| value.cwd.push_str(FOREIGN_SUFFIX),
        |value| value.evidence_path.push_str(FOREIGN_SUFFIX),
        |value| value.data_home.push_str(FOREIGN_SUFFIX),
        |value| value.process_pid = value.process_pid.map(|pid| pid + IDENTITY_INCREMENT as u32),
        |value| {
            value.process_start_time = value
                .process_start_time
                .map(|time| time + IDENTITY_INCREMENT)
        },
        |value| value.executable.push_str(FOREIGN_SUFFIX),
    ]
}
