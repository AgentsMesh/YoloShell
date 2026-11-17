use crate::{constants::*, fixture::*};
use uuid::Uuid;
use yolo_agent_contract::AgentKind;
use yolo_agent_control_contract::*;

#[test]
fn every_scope_identity_is_required_and_process_ranges_are_checked() {
    let base = witness_fields(&binding());
    let identities: &[fn(&mut NativeControlWitnessFields)] = &[
        |f| f.terminal_id = Uuid::nil(),
        |f| f.runtime_epoch = Uuid::nil(),
        |f| f.worker_id = Uuid::nil(),
    ];
    for change in identities {
        let mut fields = base.clone();
        change(&mut fields);
        assert_eq!(
            NativeControlWitness::new(fields),
            Err(ControlContractError::InvalidIdentity)
        );
    }
    let processes: &[fn(&mut NativeControlWitnessFields)] = &[
        |f| f.process_pid = u32::MIN,
        |f| f.process_pid = MAX_PROCESS_PID + STEP,
        |f| f.process_start_time = u64::MIN,
    ];
    for change in processes {
        let mut fields = base.clone();
        change(&mut fields);
        assert_eq!(
            NativeControlWitness::new(fields),
            Err(ControlContractError::InvalidProcess)
        );
    }
    let mut maximum = base;
    maximum.process_pid = MAX_PROCESS_PID;
    NativeControlWitness::new(maximum).unwrap();
}

#[test]
fn boot_identity_is_exact_bounded_and_never_normalized_from_unknown_or_padded_input() {
    let base = witness_fields(&binding());
    let unknown = format!(
        "{}{BOOT_ID}",
        yolo_process_identity::constants::UNKNOWN_BOOT_ID_PREFIX
    );
    for boot in [
        String::new(),
        unknown,
        SPACED_BOOT.into(),
        CONTROL_TEXT.into(),
        TEXT_CHARACTER.repeat(MAX_BOOT_ID_BYTES + usize::try_from(STEP).unwrap()),
    ] {
        let mut fields = base.clone();
        fields.boot_id = boot;
        assert_eq!(
            NativeControlWitness::new(fields),
            Err(ControlContractError::InvalidBoot)
        );
    }
    let mut fields = base;
    fields.boot_id = TEXT_CHARACTER.repeat(MAX_BOOT_ID_BYTES);
    let witness = NativeControlWitness::new(fields.clone()).unwrap();
    assert_eq!(witness.fields().boot_id, fields.boot_id);
}

#[test]
fn canonical_kinds_are_accepted_and_noncanonical_variants_are_rejected() {
    let base = witness_fields(&binding());
    let mut noncanonical = base.clone();
    noncanonical.kind = AgentKind::Other(yolo_agent_contract::CODEX_KIND.into());
    assert_eq!(
        NativeControlWitness::new(noncanonical),
        Err(ControlContractError::UnsupportedProvider)
    );
    for kind in [
        AgentKind::ClaudeCode,
        AgentKind::Codex,
        AgentKind::Other(OTHER_PROVIDER.into()),
    ] {
        let mut fields = base.clone();
        fields.kind = kind.clone();
        assert_eq!(
            NativeControlWitness::new(fields).unwrap().fields().kind,
            kind
        );
    }
}

#[test]
fn observation_rejects_mixed_session_provider_process_start_or_binding_digest() {
    let binding = binding();
    let changes: &[fn(&mut NativeControlWitnessFields)] = &[
        |f| f.terminal_id = Uuid::new_v4(),
        |f| f.kind = AgentKind::ClaudeCode,
        |f| f.process_pid += STEP,
        |f| f.process_start_time += TIME_STEP,
        |f| f.binding_digest = authority_digest(&REVISION).unwrap(),
    ];
    for change in changes {
        let mut fields = witness_fields(&binding);
        change(&mut fields);
        let witness = NativeControlWitness::new(fields).unwrap();
        assert_eq!(
            NativeControlObservation::new(binding.clone(), witness, false),
            Err(ControlContractError::BindingMismatch)
        );
    }
}
