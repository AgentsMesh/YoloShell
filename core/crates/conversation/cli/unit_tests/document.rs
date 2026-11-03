use crate::{arguments::Action, document::*};
use std::io::{self, Read};
use yolo_conversation_protocol::*;
use yolo_crypto::Sha256Digest;
use yolo_hub_identity::*;

struct Broken;
impl Read for Broken {
    fn read(&mut self, _: &mut [u8]) -> io::Result<usize> {
        Err(io::Error::other("unavailable input"))
    }
}

#[test]
fn agent_selection_never_reads_stdin() {
    let agent = AgentId::new();
    assert!(
        request(
            &Action::Resolve {
                agent,
                origin: None
            },
            Broken
        )
        .unwrap()
        .is_none()
    );
    assert!(
        request(
            &Action::Read {
                agent: Some(agent),
                origin: None
            },
            Broken
        )
        .unwrap()
        .is_none()
    );
}

#[test]
fn json_input_is_bounded_typed_and_validated_before_any_connection() {
    let action = Action::Watch { wait_ms: 0 };
    assert!(request(&action, Broken).is_err());
    assert!(request(&action, &[b' '; MAX_REQUEST_BYTES as usize + 1][..]).is_err());
    assert!(request(&action, &b"not json"[..]).is_err());
    let hub_id = HubId::new();
    let value = ConversationReadRequest {
        address: ConversationAddress {
            hub_id,
            conversation_id: Sha256Digest::repeated(1),
        },
        binding: ConversationBinding {
            agent: AgentTarget {
                address: AgentAddress {
                    hub_id,
                    agent_id: AgentId::new(),
                },
                execution_id: ExecutionId::new(),
            },
            runtime: RuntimeTarget {
                address: TerminalAddress {
                    hub_id,
                    terminal_id: TerminalId::new(),
                },
                runtime_epoch: RuntimeEpoch::new(),
            },
        },
        cursor: None,
        limits: ConversationLimits::default(),
        since_revision: None,
        since_generation: None,
    };
    assert_eq!(
        request(&action, serde_json::to_vec(&value).unwrap().as_slice()).unwrap(),
        Some(value.clone())
    );
    let mut invalid = value;
    invalid.since_revision = Some(Sha256Digest::repeated(2));
    assert!(request(&action, serde_json::to_vec(&invalid).unwrap().as_slice()).is_err());
}
