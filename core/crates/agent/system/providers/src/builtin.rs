use yolo_agent_provider_contract::{LaunchPolicies, ProjectionPolicies};
use yolo_agent_provider_runtime::ProviderRuntimeRegistry;
use yolo_conversation_source::ConversationSources;
use yolo_hook_installation_contract::HookInstallSpec;

const BUILTIN_RUNTIME_ERROR: &str = "built-in provider composition must be valid";

/// The only compiled-in provider composition list. Each provider package owns
/// its descriptor, ports and install profile; this list only names packages.
pub fn builtin_provider_runtime() -> ProviderRuntimeRegistry {
    ProviderRuntimeRegistry::from_runtimes([
        yolo_codex_provider::runtime(),
        yolo_claude_provider::runtime(),
        yolo_qwen_provider::runtime(),
        yolo_gemini_provider::runtime(),
        yolo_grok_provider::runtime(),
    ])
    .expect(BUILTIN_RUNTIME_ERROR)
}

/// Projection policy data for Hub processes that only need provider facts, not
/// adapter instances. Composition roots inject this into the local backend.
pub fn builtin_projection_policies() -> ProjectionPolicies {
    builtin_provider_runtime().projection_policies()
}

/// Closed launch executables derived from each provider runtime row.
pub fn builtin_launch_policies() -> LaunchPolicies {
    let mut policies = LaunchPolicies::none();
    for runtime in builtin_provider_runtime().runtimes() {
        if let Some(command) = runtime.launch_command() {
            policies.insert(runtime.descriptor().kind().clone(), command);
        }
    }
    policies
}

/// Hook installation profiles derived from the provider packages.
pub fn builtin_install_specs() -> Vec<&'static HookInstallSpec> {
    builtin_provider_runtime().install_specs()
}

pub fn builtin_install_spec(provider: &str) -> Option<&'static HookInstallSpec> {
    builtin_provider_runtime().install_spec(provider)
}

/// Conversation sources derived from the provider packages.
pub fn builtin_conversation_sources() -> ConversationSources {
    ConversationSources::new(builtin_provider_runtime().conversation_sources())
}
