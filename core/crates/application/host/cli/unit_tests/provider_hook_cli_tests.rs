use yolo_claude_provider::INSTALL_TOKEN as CLAUDE_INSTALL_TOKEN;
use yolo_codex_provider::INSTALL_TOKEN as CODEX_INSTALL_TOKEN;
use yolo_gemini_provider::INSTALL_TOKEN as GEMINI_INSTALL_TOKEN;
use yolo_grok_provider::INSTALL_TOKEN as GROK_INSTALL_TOKEN;
use yolo_qwen_provider::INSTALL_TOKEN as QWEN_INSTALL_TOKEN;
use yolo_runtime::agent::HookClient;

#[test]
fn provider_hook_clients_follow_the_composition_table() {
    for (token, client) in [
        (CODEX_INSTALL_TOKEN, HookClient::CodexNative),
        (CLAUDE_INSTALL_TOKEN, HookClient::ClaudeNative),
        (QWEN_INSTALL_TOKEN, HookClient::Ingress),
        (GEMINI_INSTALL_TOKEN, HookClient::Ingress),
        (GROK_INSTALL_TOKEN, HookClient::Ingress),
    ] {
        assert_eq!(super::provider_hook::client(token), Some(client), "{token}");
    }
    assert_eq!(super::provider_hook::client("unknown_provider"), None);
}
