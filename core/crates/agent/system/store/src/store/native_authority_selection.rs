use super::*;
use crate::agent::codex::authority::CodexAuthorityStatus;
use yolo_agent_hook_generation::HookGenerationStore;

const AMBIGUOUS: &str = "multiple native agent recovery authorities have no binding publication";
const MAX_UNPUBLISHED_AUTHORITIES: usize = 1;

pub(in crate::store) fn select(
    store: &AgentBindingStore,
    namespace: &NamespaceLease,
    current: Option<&AgentBinding>,
    runtime: AuthorityRuntimeIdentity,
) -> Result<Option<NativeAuthority>> {
    let provider_runtime = store.provider_runtime();
    let authority = |kind: &AgentKind| {
        provider_runtime
            .get(kind)
            .map(|provider| provider.descriptor().authority_kind())
    };
    let mut candidates = Vec::new();
    if current.is_none_or(|binding| {
        authority(&binding.kind) == Some(yolo_agent_providers::AuthorityKind::ClaudeGeneration)
    }) {
        let native = ClaudeNativeGenerationStore::from_binding_path(store.path())?;
        if let Some((record, identity)) = native.load_with_identity_in_namespace(namespace)? {
            let candidate = NativeAuthority::Claude(Box::new(record), identity);
            if current.map_or_else(
                || candidate.matches_runtime(runtime),
                |binding| candidate.matches_subject(binding),
            ) {
                candidates.push(candidate);
            }
        }
    }
    if current.is_none_or(|binding| {
        authority(&binding.kind) == Some(yolo_agent_providers::AuthorityKind::CodexOccurrence)
    }) {
        let codex = store.codex_authority_store()?;
        if !namespace.file_is_missing(codex.path())? {
            let (record, identity): (CodexBindingAuthority, _) =
                namespace.read_json_with_identity(codex.path())?;
            record.validate()?;
            if record.status != CodexAuthorityStatus::Revoked && record.resume_boot_id().is_some() {
                let candidate = NativeAuthority::Codex(Box::new(record), identity);
                if current.map_or_else(
                    || candidate.matches_runtime(runtime),
                    |binding| candidate.matches_subject(binding),
                ) {
                    candidates.push(candidate);
                }
            }
        }
    }
    let generic = HookGenerationStore::from_binding_path(store.path())?;
    if let Some((record, identity)) = generic.load_with_identity_in_namespace(namespace)? {
        let generic_kind = authority(&record.binding.kind)
            == Some(yolo_agent_providers::AuthorityKind::GenericHookGeneration);
        let selected = current.map_or(generic_kind, |binding| {
            authority(&binding.kind) == Some(yolo_agent_providers::AuthorityKind::GenericHookGeneration)
        });
        if selected && !record.revoked {
            let candidate = NativeAuthority::GenericHook(Box::new(record), identity);
            if current.map_or_else(
                || candidate.matches_runtime(runtime),
                |binding| candidate.matches_subject(binding),
            ) {
                candidates.push(candidate);
            }
        }
    }
    ensure!(candidates.len() <= MAX_UNPUBLISHED_AUTHORITIES, AMBIGUOUS);
    Ok(candidates.pop())
}
