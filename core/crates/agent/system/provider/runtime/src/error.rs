#[derive(Clone, Copy, Debug, Eq, PartialEq, thiserror::Error)]
pub enum ProviderRuntimeError {
    #[error("provider runtime adapter kind does not match its descriptor")]
    AdapterKindMismatch,
    #[error("provider runtime identity is duplicated")]
    DuplicateIdentity,
    #[error("provider runtime kind has no registered adapter")]
    MissingKind,
    #[error("provider runtime install token has no registered provider")]
    MissingInstallToken,
    #[error("provider runtime conversation token has no registered provider")]
    MissingConversationToken,
    #[error("provider runtime hook codec does not match its descriptor")]
    HookCodecMismatch,
    #[error("provider runtime install spec does not match its descriptor")]
    InstallSpecMismatch,
    #[error("provider runtime conversation source does not match its descriptor")]
    ConversationSourceMismatch,
    #[error("provider runtime interaction semantics do not match its authority")]
    InteractionSemanticsMismatch,
}
