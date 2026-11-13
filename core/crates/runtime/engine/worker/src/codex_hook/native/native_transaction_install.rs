use super::{CodexHookContext, NativeCodexHandoff};
use crate::agent::hooks::HookEnvelope;
use crate::codex_hooks::{
    CodexAdmissionAuthority, CodexAdmissionRecord, CodexAdmissionStorage, CodexHookArtifacts,
    CodexHookIntake, CodexNativeRequest,
};
use crate::worker::codex_hook::router::{CodexRouterError, DurableCodexAdmission};
use crate::worker::codex_hook::runtime::CodexHookRuntime;
use crate::worker::codex_hook::{native_admission, native_first_event};
use anyhow::{Context, Result, ensure};
use std::sync::Arc;

const NATIVE_ROUTE_ABSENT: &str = "native Codex SessionStart is required before other callbacks";
const NATIVE_ADMISSION_CONFLICT: &str = "native Codex admission does not match the exact agent";
const NATIVE_GENERATION_REVOKED: &str = "native Codex callback generation has been revoked";
const ADMISSION_RECOVERY: &str = "recover native Codex admission authority";
const ADMISSION_CAPTURE: &str = "capture native Codex admission authority";
const ADMISSION_REVALIDATION: &str = "revalidate native Codex admission process and evidence";
const WRITER_CAPTURE: &str = "capture native Codex retirement writer";
const FIRST_EVENT_PRIME: &str = "prime native Codex admission first event";
const REGISTRATION_COMPLETE: &str = "complete native Codex admission registration";

pub(super) fn install(
    transaction: &CodexHookContext,
    peer: crate::process::ProcessIdentity,
    request: CodexNativeRequest,
    admitted: native_admission::NativeAdmission,
    verify_request: impl Fn() -> Result<()>,
) -> Result<NativeCodexHandoff> {
    let context = &admitted.context;
    let identity = &context.identity;
    let artifacts = CodexHookArtifacts::new(
        &transaction.paths,
        identity.terminal_id,
        identity.runtime_epoch,
        identity.invocation_id,
    )?;
    let provisional = CodexAdmissionStorage::for_identity(&artifacts, identity)?;
    let publication = provisional.lock_publication()?;
    let existing =
        CodexAdmissionAuthority::recover_artifacts(&artifacts).context(ADMISSION_RECOVERY)?;
    let record = match &existing {
        Some(admission) => {
            let record = admission.record();
            let previous = record.request();
            ensure!(
                previous.identity.process == identity.process
                    && previous.identity.invocation_id == identity.invocation_id
                    && previous.identity.terminal_id == identity.terminal_id
                    && previous.identity.runtime_epoch == identity.runtime_epoch
                    && previous.hook_executable == context.hook_executable
                    && previous.real_executable == context.real_executable
                    && previous.data_home == context.data_home,
                NATIVE_ADMISSION_CONFLICT
            );
            record.clone()
        }
        None => {
            ensure!(request.is_session_start(), NATIVE_ROUTE_ABSENT);
            let first = admitted.envelope(&request, identity)?;
            let mut reducer = crate::agent::hooks::Reducer::new(
                identity.clone(),
                crate::agent::hooks::HookProviderKind::Codex,
                context.provider_version,
            )
            .map_err(anyhow::Error::new)?;
            let mut admitted_first = first.clone();
            admitted_first.sequence =
                crate::agent::hooks::EventSequence(crate::agent::hooks::FIRST_SEQUENCE);
            reducer.apply(admitted_first).map_err(anyhow::Error::new)?;
            let revision = transaction
                .router
                .reserve_publication_revision()
                .map_err(anyhow::Error::new)?;
            CodexAdmissionRecord::capture_native_with_route_revision(
                context.clone(),
                revision,
                first,
            )
            .context(ADMISSION_CAPTURE)?
        }
    };
    let owner = &record.request().identity;
    let storage = CodexAdmissionStorage::for_identity(&artifacts, owner)?;
    let writer = storage
        .acquire_retirement_writer_fence()
        .context(WRITER_CAPTURE)?;
    writer.verify_unsealed()?;
    CodexHookIntake::for_identity(artifacts.intake_directory(), owner)?;
    let mut generation_fence = None;
    let installed = storage.publish_with_commit_fence(&record, || {
        verify_request()?;
        writer.verify_unsealed()?;
        admitted
            .verify(
                transaction.process_source.as_ref(),
                transaction.shell,
                peer,
                &request,
                &transaction.policy,
                &record,
            )
            .context(ADMISSION_REVALIDATION)?;
        if generation_fence.is_none() {
            generation_fence = Some(
                transaction
                    .generation
                    .try_fence()
                    .ok_or_else(|| anyhow::anyhow!(NATIVE_GENERATION_REVOKED))?,
            );
        }
        Ok(())
    })?;
    let admission =
        DurableCodexAdmission::from_publication(storage, installed).map_err(anyhow::Error::new)?;
    native_first_event::prime(&transaction.paths, admission.authority())
        .context(FIRST_EVENT_PRIME)?;
    drop(publication);
    let lease = transaction
        .router
        .install_or_replay(admission, |record| {
            let request = record.request();
            CodexHookRuntime::with_process_source_generation(
                &transaction.paths,
                Arc::clone(&transaction.registry),
                request.identity.terminal_id,
                request.identity.runtime_epoch,
                transaction.shell,
                request.identity.clone(),
                request.provider_version,
                Arc::clone(&transaction.process_source),
                Some(transaction.generation.clone()),
            )
            .map(|(runtime, _)| runtime)
            .map_err(|_| CodexRouterError::Runtime)
        })
        .map_err(anyhow::Error::new)?;
    verify_request()?;
    writer.verify_unsealed()?;
    lease
        .entry()
        .runtime()
        .complete_native_registration(lease.entry().admission().authority())
        .context(REGISTRATION_COMPLETE)?;
    let mut event = admitted.envelope(&request, &record.request().identity)?;
    let first_event = same_occurrence(record.first_event(), &event);
    if first_event {
        event = record.first_event().clone();
    }
    Ok(NativeCodexHandoff {
        lease,
        event,
        first_event,
    })
}

fn same_occurrence(first: &HookEnvelope, incoming: &HookEnvelope) -> bool {
    if incoming.event_id != first.event_id
        && !matches!(
            incoming.event,
            crate::agent::hooks::EventKind::SessionStart {
                source: crate::agent::hooks::SessionStartSource::Startup
            }
        )
    {
        return false;
    }
    let mut normalized = incoming.clone();
    normalized.event_id = first.event_id;
    normalized.sequence = first.sequence;
    normalized == *first
}
