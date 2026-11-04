use super::constants::INVALID;

mod evidence;

use evidence::{generation_request, inspect_evidence};
use super::contract::HookRequest;
use super::host::Context;
use anyhow::{Context as _, Result, ensure};
use std::path::Path;
use yolo_agent_adapter_contract::AgentAdapter;
use yolo_agent_authority_contract::AuthorityProcessIdentity;
use yolo_agent_contract::AgentBinding;
use yolo_agent_hook_generation::{
    GenerationEvidence, GenerationRequest, GenerationScope, HookGenerationRecord,
    HookGenerationStore, HookMetadata, HookProviderKind,
};
use yolo_hooks_runtime::EventKind;
use yolo_paths::identity::ObjectIdentity;

pub(super) enum Outcome {
    Ignored,
    Applied(HookGenerationRecord),
}

pub(super) async fn execute(
    context: &Context,
    peer_pid: u32,
    request: &HookRequest,
) -> Result<Outcome> {
    let host = &context.host;
    let _gate = host.generation_gate.lock().await;
    ensure!(
        request.is_valid()
            && request.scope.terminal_id == host.terminal_id
            && request.scope.runtime_epoch == host.runtime_epoch,
        INVALID
    );
    host.namespace.verify()?;
    ensure!((host.runtime_current)(), INVALID);
    let codec = host.codec(&request.provider).context(INVALID)?;
    let profile = codec.profile();
    let adapter = host
        .provider_runtime
        .adapter(&codec.kind())
        .context(INVALID)?;
    let Some(mut event) = codec.decode(&request.payload)? else {
        return Ok(Outcome::Ignored);
    };
    let store = HookGenerationStore::from_binding_path(&host.binding_path)?;
    let admitted = super::admission::capture(
        peer_pid,
        &profile,
        host.shell,
        request.scope,
        &context.executable,
        host.process_source.as_ref(),
        adapter,
    )?;
    // Admission has identified the live agent, so a provider that omits the
    // transcript path on a fresh session's start can derive it now.
    event.transcript_path = codec.resolve_transcript_path(&admitted.agent, &event)?;
    let evidence = inspect_evidence(&admitted, &event, request, codec.as_ref(), adapter)?;
    let generation = generation_request(request, codec.as_ref(), &event);
    let mut commit_fence = || {
        ensure!(
            (host.runtime_current)()
                && super::admission::capture(
                    peer_pid,
                    &profile,
                    host.shell,
                    request.scope,
                    &context.executable,
                    host.process_source.as_ref(),
                    adapter,
                )? == admitted
                && super::outbox::next(&host.outbox, request.scope)?.as_ref() == Some(request)
                && evidence.verify_own()?,
            INVALID
        );
        host.namespace.verify()
    };
    let record = store.apply_in_namespace(
        host.namespace.as_ref(),
        &generation,
        host.worker,
        &host.boot_id,
        &evidence,
        &mut commit_fence,
    )?;
    let previous = read_binding(host.namespace.as_ref(), &host.binding_path)?;
    store.publish_binding_in_namespace(host.namespace.as_ref(), &record, &mut commit_fence)?;
    publish_binding(&host.publish, previous.as_ref(), &record);
    Ok(Outcome::Applied(record))
}

fn read_binding(
    namespace: &yolo_paths::namespace_lease::NamespaceLease,
    path: &Path,
) -> Result<Option<AgentBinding>> {
    // The binding file owner maps a concurrent removal to `None`; a separate
    // missing-check would leave a window that turns the same race into an error.
    yolo_agent_binding_io::AgentBindingFile::new(path).read_in_namespace(namespace)
}

fn publish_binding(
    publish: &std::sync::Arc<dyn Fn(&'static str, AgentBinding) + Send + Sync>,
    previous: Option<&AgentBinding>,
    record: &HookGenerationRecord,
) {
    if record.revoked {
        return;
    }
    let binding = record.recovery_binding(true);
    if previous == Some(&binding) {
        return;
    }
    let identity_changed = previous.is_none_or(|previous| {
        previous.kind != binding.kind
            || previous.provider_session_id != binding.provider_session_id
            || previous.executable != binding.executable
            || previous.cwd != binding.cwd
            || previous.evidence_path != binding.evidence_path
            || previous.data_home != binding.data_home
    });
    publish(
        if identity_changed {
            yolo_wire::event::AGENT_BINDING_CHANGED
        } else {
            yolo_wire::event::AGENT_STATUS_CHANGED
        },
        binding,
    );
}
