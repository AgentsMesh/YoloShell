use crate::fixture::*;
use yolo_agent_contract::AgentState;
use yolo_agent_control_contract::*;

#[test]
fn a_provider_turn_end_extends_readiness_to_a_stop_candidate() {
    let mut binding = binding();
    binding.state = AgentState::Active;
    let witness = NativeControlWitness::new(witness_fields(&binding)).unwrap();
    let end = NativeTurnEnd {
        turn_id: "turn-1".into(),
        kind: NativeTurnEndKind::Completed,
    };
    let observed = NativeControlObservation::new_with_turn_end(
        binding.clone(),
        witness.clone(),
        true,
        Some(end.clone()),
    )
    .unwrap();
    assert!(observed.prompt_input_ready());
    assert_eq!(observed.turn_end(), Some(&end));
    assert_eq!(
        serde_json::from_value::<NativeControlObservation>(
            serde_json::to_value(&observed).unwrap()
        )
        .unwrap(),
        observed
    );
    assert!(
        NativeControlObservation::new_with_turn_end(binding.clone(), witness.clone(), true, None)
            .is_err()
    );
    assert!(
        NativeControlObservation::new_with_turn_end(
            binding.clone(),
            witness.clone(),
            false,
            Some(end.clone())
        )
        .is_err()
    );
    let mut claude = binding;
    claude.kind = yolo_agent_contract::AgentKind::ClaudeCode;
    let claude_witness = NativeControlWitness::new(witness_fields(&claude)).unwrap();
    assert!(
        NativeControlObservation::new_with_turn_end(
            claude.clone(),
            claude_witness.clone(),
            true,
            Some(end.clone())
        )
        .is_ok()
    );
    assert!(
        NativeControlObservation::new_with_turn_end(claude, claude_witness, true, None).is_err()
    );
}

#[test]
fn readiness_requires_the_same_waiting_binding_and_is_required_on_the_wire() {
    let mut binding = binding();
    for state in [
        AgentState::Active,
        AgentState::Running,
        AgentState::WaitingForPermission,
        AgentState::WaitingForInput,
    ] {
        binding.state = state.clone();
        let witness = NativeControlWitness::new(witness_fields(&binding)).unwrap();
        let observed = NativeControlObservation::new(binding.clone(), witness.clone(), true);
        if state == AgentState::WaitingForInput {
            let observed = observed.unwrap();
            assert!(observed.prompt_input_ready());
            let json = serde_json::to_value(&observed).unwrap();
            assert_eq!(
                serde_json::from_value::<NativeControlObservation>(json).unwrap(),
                observed
            );
        } else {
            assert_eq!(observed, Err(ControlContractError::BindingMismatch));
        }
        let unavailable = NativeControlObservation::new(binding.clone(), witness, false).unwrap();
        assert!(!unavailable.prompt_input_ready());
        let mut missing = serde_json::to_value(unavailable).unwrap();
        missing
            .as_object_mut()
            .unwrap()
            .remove(PROMPT_INPUT_READY_FIELD);
        assert!(serde_json::from_value::<NativeControlObservation>(missing).is_err());
    }
}
