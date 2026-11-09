use uuid::Uuid;
use yolo_hub_agent_protocol::*;

use crate::constants::*;
use crate::fixtures::*;

#[test]
fn interaction_kinds_and_choice_boundaries_round_trip() {
    for kind in [
        InteractionKind::Approval,
        InteractionKind::Choice,
        InteractionKind::Text,
    ] {
        let pending = interaction(target(), kind);
        assert_eq!(pending.kind(), kind);
        assert_eq!(round_trip(&pending), pending);
    }
    let reference = InteractionReference {
        target: target(),
        id: Uuid::new_v4(),
    };
    let mut choices = Vec::new();
    for index in 0..MAX_INTERACTION_CHOICES {
        choices.push(InteractionChoice {
            id: index.to_string(),
            label: TEXT_CHARACTER.repeat(MAX_CHOICE_LABEL_BYTES),
        });
    }
    let full = Interaction::new(reference, InteractionKind::Choice, choices.clone()).unwrap();
    assert_eq!(full.reference(), reference);
    assert_eq!(full.choices(), choices);
    choices.push(choice());
    assert_eq!(
        Interaction::new(reference, InteractionKind::Choice, choices),
        Err(ProtocolError::InvalidInteraction)
    );
}

#[test]
fn interaction_prompt_round_trips_and_is_bounded() {
    let reference = interaction(target(), InteractionKind::Approval).reference();
    let pending = Interaction::with_prompt(
        reference,
        InteractionKind::Approval,
        vec![choice()],
        "Bash: cargo test".into(),
    )
    .unwrap();
    assert_eq!(pending.prompt(), "Bash: cargo test");
    assert_eq!(round_trip(&pending), pending);
    for invalid in [
        "x".repeat(MAX_INTERACTION_PROMPT_BYTES + SEQUENCE_STEP as usize),
        "line\nbreak\0".into(),
    ] {
        assert_eq!(
            Interaction::with_prompt(
                reference,
                InteractionKind::Approval,
                vec![choice()],
                invalid
            ),
            Err(ProtocolError::InvalidInteraction)
        );
    }
}

#[test]
fn interaction_rejects_nil_identity_wrong_kind_and_duplicate_choices() {
    let valid = interaction(target(), InteractionKind::Approval);
    let reference = valid.reference();
    let mut nil = reference;
    nil.id = Uuid::nil();
    assert_eq!(
        Interaction::new(nil, InteractionKind::Approval, vec![choice()]),
        Err(ProtocolError::InvalidInteraction)
    );
    for kind in [InteractionKind::Approval, InteractionKind::Choice] {
        assert_eq!(
            Interaction::new(reference, kind, Vec::new()),
            Err(ProtocolError::InvalidInteraction)
        );
    }
    assert_eq!(
        Interaction::new(reference, InteractionKind::Text, vec![choice()]),
        Err(ProtocolError::InvalidInteraction)
    );
    assert_eq!(
        Interaction::new(
            reference,
            InteractionKind::Approval,
            vec![choice(), choice()]
        ),
        Err(ProtocolError::InvalidInteraction)
    );
    let mut wire = serde_json::to_value(&valid).unwrap();
    wire["reference"]["id"] = Uuid::nil().to_string().into();
    assert!(serde_json::from_value::<Interaction>(wire).is_err());
}

#[test]
fn interaction_validates_choice_id_and_label_separately() {
    let reference = interaction(target(), InteractionKind::Choice).reference();
    for invalid in [
        String::new(),
        INVALID_PROVIDER.into(),
        TEXT_CHARACTER.repeat(MAX_CHOICE_ID_BYTES + SEQUENCE_STEP as usize),
    ] {
        let candidate = InteractionChoice {
            id: invalid,
            label: CHOICE_LABEL.into(),
        };
        assert_eq!(
            Interaction::new(reference, InteractionKind::Choice, vec![candidate]),
            Err(ProtocolError::InvalidInteraction)
        );
    }
    for invalid in [
        String::new(),
        "line\nbreak".into(),
        TEXT_CHARACTER.repeat(MAX_CHOICE_LABEL_BYTES + SEQUENCE_STEP as usize),
    ] {
        let candidate = InteractionChoice {
            id: CHOICE_ID.into(),
            label: invalid,
        };
        assert_eq!(
            Interaction::new(reference, InteractionKind::Choice, vec![candidate]),
            Err(ProtocolError::InvalidInteraction)
        );
    }
    let boundary = InteractionChoice {
        id: TEXT_CHARACTER.repeat(MAX_CHOICE_ID_BYTES),
        label: CHOICE_LABEL.into(),
    };
    assert!(Interaction::new(reference, InteractionKind::Choice, vec![boundary]).is_ok());
}
