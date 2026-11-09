use super::*;
use serde_json::json;

const INVALID_AUTHORITY: &str = "Codex binding authority is invalid";
const REVISION_STEP: u64 = 1;

#[test]
fn every_persisted_codex_authority_identity_and_version_field_is_required() {
    let fixture = Fixture::new();
    let original = fixture.authority(Generation::INITIAL.0, CodexAuthorityStatus::Active);
    let values = [
        (
            stringify!(schema_version),
            json!(AUTHORITY_SCHEMA_VERSION + 1),
        ),
        (stringify!(runtime_epoch), json!(Uuid::nil())),
        (stringify!(worker_pid), json!(0)),
        (stringify!(worker_start_time), json!(0)),
        (stringify!(provider_pid), json!(0)),
        (stringify!(provider_start_time), json!(0)),
        (
            stringify!(provider_version),
            json!(crate::agent::hooks::ProviderVersion::new(u16::MAX, 0, 0)),
        ),
    ];
    for (field, replacement) in values {
        let mut value = serde_json::to_value(&original).unwrap();
        value[field] = replacement;
        let changed: CodexBindingAuthority = serde_json::from_value(value).unwrap();
        assert_eq!(
            changed.validate().unwrap_err().to_string(),
            INVALID_AUTHORITY,
            "{field}"
        );
        assert!(
            !changed.authorizes(&original.binding, fixture.runtime),
            "{field}"
        );
    }
    let mutations: &[(&str, fn(&mut CodexBindingAuthority))] = &[
        (stringify!(route_revision), |value| {
            value.version.route_revision = u64::default()
        }),
        (stringify!(invocation_id), |value| {
            value.version.invocation_id = Uuid::nil()
        }),
        (stringify!(generation), |value| {
            value.version.generation = Generation(u64::default())
        }),
        (stringify!(authority_revision), |value| {
            value.version.authority_revision = u64::default()
        }),
        (stringify!(authority_revision_max), |value| {
            value.version.authority_revision = u64::MAX
        }),
        (stringify!(kind), |value| {
            value.binding.kind = AgentKind::ClaudeCode
        }),
        (stringify!(terminal_id), |value| {
            value.binding.terminal_id = Uuid::nil()
        }),
        (stringify!(process_pid), |value| {
            value.binding.process_pid = Some(PROVIDER_PID + 1)
        }),
        (stringify!(process_start_time), |value| {
            value.binding.process_start_time = Some(PROVIDER_START + REVISION_STEP)
        }),
    ];
    for (field, mutate) in mutations {
        let mut changed = original.clone();
        mutate(&mut changed);
        assert_eq!(
            changed.validate().unwrap_err().to_string(),
            INVALID_AUTHORITY,
            "{field}"
        );
        assert!(
            !changed.authorizes(&original.binding, fixture.runtime),
            "{field}"
        );
    }
    assert!(original.authorizes(&original.binding, fixture.runtime));
}

#[test]
fn codex_status_activity_and_recovery_availability_form_an_exact_state_matrix() {
    let fixture = Fixture::new();
    for status in [
        CodexAuthorityStatus::Active,
        CodexAuthorityStatus::Exhausted,
        CodexAuthorityStatus::Unavailable,
        CodexAuthorityStatus::Revoked,
    ] {
        for state in [
            AgentState::Active,
            AgentState::Running,
            AgentState::WaitingForInput,
            AgentState::WaitingForPermission,
            AgentState::Unknown,
            AgentState::Exited,
        ] {
            for availability in [
                AgentRecoveryAvailability::Available,
                AgentRecoveryAvailability::Unavailable,
                AgentRecoveryAvailability::UnmanagedLaunch,
            ] {
                let mut value = fixture.authority(Generation::INITIAL.0, status);
                value.binding.state = state;
                value.binding.recovery_availability = availability;
                if state == AgentState::Exited {
                    value.binding.process_pid = None;
                    value.binding.process_start_time = None;
                }
                let valid = if status == CodexAuthorityStatus::Active {
                    !matches!(state, AgentState::Unknown | AgentState::Exited)
                        && availability == AgentRecoveryAvailability::Available
                } else {
                    state == AgentState::Unknown
                        && availability == AgentRecoveryAvailability::Unavailable
                };
                assert_eq!(
                    value.validate().is_ok(),
                    valid,
                    "{status:?} {state:?} {availability:?}"
                );
            }
        }
    }
}

#[test]
fn terminal_authority_requires_both_process_identity_fields_to_be_absent() {
    let fixture = Fixture::new();
    let mut original = fixture.verified_authority(Generation::INITIAL.0);
    original.binding.state = AgentState::Exited;
    original.binding.process_pid = None;
    original.binding.process_start_time = None;
    original.validate().unwrap();
    for pid_field in [true, false] {
        let mut changed = original.clone();
        if pid_field {
            changed.binding.process_pid = Some(PROVIDER_PID);
        } else {
            changed.binding.process_start_time = Some(PROVIDER_START);
        }
        assert!(changed.validate().is_err());
    }
}

#[test]
fn resume_identity_field_is_required_but_active_null_remains_valid() {
    let fixture = Fixture::new();
    let authority = fixture.authority(Generation::INITIAL.0, CodexAuthorityStatus::Active);
    let mut value = serde_json::to_value(&authority).unwrap();
    let decoded: CodexBindingAuthority = serde_json::from_value(value.clone()).unwrap();
    decoded.validate().unwrap();
    assert_eq!(decoded, authority);
    value
        .as_object_mut()
        .unwrap()
        .remove(stringify!(resume_identity));
    assert!(serde_json::from_value::<CodexBindingAuthority>(value).is_err());
}
