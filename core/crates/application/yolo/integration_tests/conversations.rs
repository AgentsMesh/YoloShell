use super::{command, constants::*};

const EXAMPLE_HUB: &str = "c5811aba-fcf6-4bdb-80a8-fa47c082f9fe";
const ABSENT_ENTRY: &str = "/nonexistent-yolo-conversation-entry.sock";

#[tokio::test]
async fn single_binary_exposes_read_only_conversation_commands_without_a_tty() {
    let help = command::run(&[CONVERSATION_COMMAND, HELP], None).await;
    assert_eq!(help.code, SUCCESS, "{}", help.stderr);
    for action in [CONVERSATION_RESOLVE, CONVERSATION_READ, CONVERSATION_WATCH] {
        assert!(help.stdout.contains(action));
    }
    assert!(help.stderr.is_empty());
    let incomplete = command::run(&[CONVERSATION_COMMAND, CONVERSATION_RESOLVE], None).await;
    assert_eq!(incomplete.code, USAGE);
    assert!(incomplete.stdout.is_empty());
}

#[tokio::test]
async fn missing_typed_input_fails_before_connecting_or_starting_any_service() {
    let result = command::run(
        &[
            CONVERSATION_COMMAND,
            CONVERSATION_HUB_OPTION,
            EXAMPLE_HUB,
            CONVERSATION_SOCKET_OPTION,
            ABSENT_ENTRY,
            CONVERSATION_READ,
        ],
        None,
    )
    .await;
    assert_eq!(result.code, FAILURE);
    assert!(result.stdout.is_empty());
    assert!(result.stderr.contains("ConversationReadRequest"));
    assert!(!std::path::Path::new(ABSENT_ENTRY).exists());
}
