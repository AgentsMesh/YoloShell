use super::{command, constants::*};

const EXAMPLE_HUB: &str = "c5811aba-fcf6-4bdb-80a8-fa47c082f9fe";
const ABSENT_ENTRY: &str = "/nonexistent-yolo-agent-entry.sock";

#[tokio::test]
async fn single_binary_exposes_agent_commands_without_a_tty() {
    let help = command::run(&[AGENT_COMMAND, HELP], None).await;
    assert_eq!(help.code, SUCCESS, "{}", help.stderr);
    for action in [
        AGENT_LIST,
        AGENT_GET,
        AGENT_SEND,
        AGENT_ASK,
        AGENT_INTERRUPT,
        AGENT_READ,
    ] {
        assert!(help.stdout.contains(action));
    }
    assert!(help.stderr.is_empty());
    let incomplete = command::run(&[AGENT_COMMAND], None).await;
    assert_eq!(incomplete.code, USAGE);
    assert!(incomplete.stdout.is_empty());
}

#[tokio::test]
async fn an_absent_entry_socket_fails_before_starting_any_service() {
    let result = command::run(
        &[
            AGENT_COMMAND,
            AGENT_LIST,
            CONVERSATION_HUB_OPTION,
            EXAMPLE_HUB,
            CONVERSATION_SOCKET_OPTION,
            ABSENT_ENTRY,
        ],
        None,
    )
    .await;
    assert_eq!(result.code, FAILURE);
    assert!(result.stdout.is_empty());
    assert!(!std::path::Path::new(ABSENT_ENTRY).exists());
}
