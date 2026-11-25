use super::*;
use crate::claude::generation::termination::SessionEndBoundary;

#[test]
fn each_event_variant_requires_its_own_identity_state_and_transcript_contract() {
    let fixture = Fixture::new();
    let id = Uuid::new_v4().to_string();
    let start = fixture.start(&id, SessionStartSource::Fresh);
    let GenerationEvent::SessionStart {
        cwd,
        transcript_path,
        ..
    } = start.clone()
    else {
        unreachable!()
    };
    let events = [
        start,
        GenerationEvent::StateChanged {
            event_id: Uuid::new_v4(),
            provider_session_id: id.clone(),
            state: AgentState::Running,
        },
        GenerationEvent::CwdChanged {
            event_id: Uuid::new_v4(),
            provider_session_id: id.clone(),
            cwd,
            transcript_path,
        },
        end(&id),
    ];
    for original in events {
        validation::validate_event(&original, &fixture.begin.data_home).unwrap();
        let fields = match original {
            GenerationEvent::SessionStart { .. } => vec![
                stringify!(event_id),
                stringify!(provider_session_id),
                stringify!(cwd),
                stringify!(transcript_path),
                stringify!(state),
            ],
            GenerationEvent::StateChanged { .. } => vec![
                stringify!(event_id),
                stringify!(provider_session_id),
                stringify!(state),
            ],
            GenerationEvent::CwdChanged { .. } => vec![
                stringify!(event_id),
                stringify!(provider_session_id),
                stringify!(cwd),
                stringify!(transcript_path),
            ],
            GenerationEvent::SessionEnd { .. } => vec![
                stringify!(event_id),
                stringify!(provider_session_id),
                stringify!(boundary),
            ],
        };
        for field in fields {
            let mut value = serde_json::to_value(&original).unwrap();
            value[field] = match field {
                stringify!(event_id) => serde_json::json!(Uuid::nil()),
                stringify!(state) => serde_json::json!(AgentState::Exited),
                stringify!(boundary) => serde_json::json!(SessionEndBoundary::Recovering),
                _ => serde_json::json!("invalid"),
            };
            let changed: GenerationEvent = serde_json::from_value(value).unwrap();
            assert_eq!(
                validation::validate_event(&changed, &fixture.begin.data_home)
                    .unwrap_err()
                    .to_string(),
                AUTHORITY_EVENT_INVALID,
                "{field}"
            );
        }
    }
}
