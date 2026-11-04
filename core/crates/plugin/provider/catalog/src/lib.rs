//! Origin-owned provider declarations. Provider identity is owned by each
//! provider package; this catalog only exposes those declarations and never
//! instantiates provider runtime code.
use yolo_plugin_provider::ProviderDefinition;

/// Compiled-in provider declarations, in stable discovery order.
pub fn definitions() -> Vec<ProviderDefinition> {
    vec![
        yolo_codex_identity::definition(),
        yolo_claude_identity::definition(),
        yolo_qwen_identity::definition(),
        yolo_gemini_identity::definition(),
        yolo_grok_identity::definition(),
    ]
}

pub fn provider_ids() -> Vec<String> {
    definitions()
        .iter()
        .map(|definition| definition.id().to_owned())
        .collect()
}

pub fn get(id: &str) -> Option<ProviderDefinition> {
    definitions()
        .into_iter()
        .find(|definition| definition.id() == id)
}
