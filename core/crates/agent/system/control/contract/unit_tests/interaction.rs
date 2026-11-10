use crate::fixture::{binding, observation, witness_fields};
use uuid::Uuid;
use yolo_agent_control_contract::*;

fn choice(id: &str, label: &str) -> NativeInteractionChoice {
    NativeInteractionChoice {
        id: id.into(),
        label: label.into(),
    }
}

fn approval() -> NativeInteraction {
    NativeInteraction::new(
        Uuid::new_v4(),
        NativeInteractionKind::Approval,
        vec![choice("allow", "Allow"), choice("deny", "Deny")],
        "Bash: cargo test".into(),
    )
    .unwrap()
}

#[test]
fn interaction_validates_identity_prompt_and_choices() {
    assert_eq!(
        NativeInteraction::new(
            Uuid::nil(),
            NativeInteractionKind::Approval,
            vec![choice("allow", "Allow")],
            String::new(),
        ),
        Err(ControlContractError::InvalidInteraction)
    );
    for (kind, choices) in [
        (NativeInteractionKind::Approval, Vec::new()),
        (NativeInteractionKind::Text, vec![choice("allow", "Allow")]),
    ] {
        assert_eq!(
            NativeInteraction::new(Uuid::new_v4(), kind, choices, String::new()),
            Err(ControlContractError::InvalidInteraction)
        );
    }
    assert_eq!(
        NativeInteraction::new(
            Uuid::new_v4(),
            NativeInteractionKind::Approval,
            vec![choice("allow", "Allow"), choice("allow", "Again")],
            String::new(),
        ),
        Err(ControlContractError::InvalidInteraction)
    );
    assert_eq!(
        NativeInteraction::new(
            Uuid::new_v4(),
            NativeInteractionKind::Approval,
            vec![choice("allow", "Allow")],
            "x".repeat(MAX_INTERACTION_PROMPT_BYTES + 1),
        ),
        Err(ControlContractError::InvalidInteraction)
    );
}

#[test]
fn observation_carries_a_pending_interaction_and_round_trips() {
    let binding = binding();
    let witness = NativeControlWitness::new(witness_fields(&binding)).unwrap();
    let interaction = approval();
    let expected = NativeControlObservation::new_with_interaction(
        binding,
        witness,
        false,
        None,
        Some(interaction.clone()),
    )
    .unwrap();
    assert_eq!(expected.interaction(), Some(&interaction));
    let encoded = serde_json::to_value(&expected).unwrap();
    assert_eq!(
        serde_json::from_value::<NativeControlObservation>(encoded).unwrap(),
        expected
    );
    assert!(observation().interaction().is_none());
}
