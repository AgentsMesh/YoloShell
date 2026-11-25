use uuid::Uuid;
use yolo_terminal_model::{CreateTerminalResult, CreationQueryOutcome, CreationQueryResult};

#[test]
fn query_outcomes_round_trip_and_remain_bound_to_the_requested_token() {
    let creation = CreateTerminalResult {
        terminal_id: Uuid::new_v4(),
        creation_capability: Uuid::new_v4(),
    };
    for outcome in [
        CreationQueryOutcome::Missing {},
        CreationQueryOutcome::Pending {},
        CreationQueryOutcome::Ready { creation },
        CreationQueryOutcome::Finalized { creation },
        CreationQueryOutcome::Survivor { creation },
        CreationQueryOutcome::Archived {
            terminal_id: creation.terminal_id,
        },
    ] {
        let result = CreationQueryResult {
            creation_token: Uuid::new_v4(),
            outcome,
        };
        let decoded: CreationQueryResult =
            serde_json::from_slice(&serde_json::to_vec(&result).unwrap()).unwrap();
        assert_eq!(decoded, result);
        assert!(decoded.matches_request(result.creation_token));
        assert!(!decoded.matches_request(Uuid::new_v4()));
        assert!(!decoded.matches_request(Uuid::nil()));
    }
}

#[test]
fn query_diagnostics_redact_token_and_creation_capability() {
    let token = Uuid::new_v4();
    let capability = Uuid::new_v4();
    let result = CreationQueryResult {
        creation_token: token,
        outcome: CreationQueryOutcome::Ready {
            creation: CreateTerminalResult {
                terminal_id: Uuid::new_v4(),
                creation_capability: capability,
            },
        },
    };
    let text = format!("{result:?}");
    assert!(!text.contains(&token.to_string()));
    assert!(!text.contains(&capability.to_string()));
}

#[test]
fn nil_result_identities_do_not_pass_transport_query_validation() {
    let token = Uuid::new_v4();
    for creation in [
        CreateTerminalResult {
            terminal_id: Uuid::nil(),
            creation_capability: Uuid::new_v4(),
        },
        CreateTerminalResult {
            terminal_id: Uuid::new_v4(),
            creation_capability: Uuid::nil(),
        },
    ] {
        let result = CreationQueryResult {
            creation_token: token,
            outcome: CreationQueryOutcome::Ready { creation },
        };
        assert!(!result.matches_request(token));
    }
}
