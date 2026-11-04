/// Which client the unified `provider-hook` command runs for this provider.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum HookClient {
    /// The provider-neutral ingress client (the default).
    #[default]
    Ingress,
    /// Codex's occurrence-witness client.
    CodexNative,
    /// Claude's native generation client.
    ClaudeNative,
}
