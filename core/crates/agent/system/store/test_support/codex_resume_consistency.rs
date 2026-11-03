use super::{CodexBindingAuthority, Fixture};
use crate::{AgentBinding, AgentKind};
use std::path::{Path, PathBuf};

#[path = "codex_resume_consistency_constants.rs"]
mod constants;
pub(super) use constants::*;

#[derive(Clone, Copy, Debug)]
pub(super) enum BindingChange {
    ShellOwner,
    ProviderSession,
    WorkingDirectory,
    DataHome,
    TranscriptPath,
    DetectionTime,
}

#[derive(Clone, Copy, Debug)]
pub(super) enum SourceChange {
    Boot,
    TranscriptPath,
    DataHomePath,
    TranscriptObject,
    DirectoryObject,
    DataHomeObject,
}

pub(super) fn original(fixture: &Fixture) -> CodexBindingAuthority {
    fixture
        .store
        .codex_authority_store()
        .unwrap()
        .load_in_namespace(&fixture.namespace)
        .unwrap()
        .unwrap()
}

pub(super) fn rebuild(
    fixture: &Fixture,
    original: &CodexBindingAuthority,
    binding: AgentBinding,
    advance: bool,
) -> CodexBindingAuthority {
    let mut version = original.version;
    if advance {
        version.authority_revision += REVISION_STEP;
        version.sequence += REVISION_STEP;
    }
    CodexBindingAuthority::new(
        fixture.runtime,
        original.provider_process(),
        crate::codex_hooks::ACCEPTED_PROVIDER_VERSION,
        version,
        original.status,
        binding,
    )
    .unwrap()
}

pub(super) fn change_binding(binding: &mut AgentBinding, change: BindingChange) {
    match change {
        BindingChange::ShellOwner => binding.terminal_id = uuid::Uuid::new_v4(),
        BindingChange::ProviderSession => {
            binding.provider_session_id = uuid::Uuid::new_v4().to_string();
            binding.evidence_path = rollout(binding).display().to_string();
        }
        BindingChange::WorkingDirectory => {
            let path = Path::new(&binding.cwd).with_file_name(OTHER_CWD);
            std::fs::create_dir(&path).unwrap();
            binding.cwd = path.display().to_string();
        }
        BindingChange::DataHome => {
            let path = Path::new(&binding.data_home).with_file_name(OTHER_HOME);
            std::fs::create_dir(&path).unwrap();
            binding.data_home = path.display().to_string();
            binding.evidence_path = rollout(binding).display().to_string();
        }
        BindingChange::TranscriptPath => {
            binding.evidence_path = rollout(binding).display().to_string()
        }
        BindingChange::DetectionTime => {
            binding.detected_at_ms += TIMESTAMP_STEP;
            binding.updated_at_ms += TIMESTAMP_STEP;
        }
    }
}

pub(super) fn change_source(binding: &mut AgentBinding, change: SourceChange) {
    match change {
        SourceChange::Boot => {}
        SourceChange::TranscriptPath => {
            let previous = PathBuf::from(&binding.evidence_path);
            let next = rollout(binding);
            // Preserve the original transcript object under a different legal
            // rollout pathname; the source path itself must still be fenced.
            std::fs::remove_file(&next).unwrap();
            std::fs::rename(previous, &next).unwrap();
            binding.evidence_path = next.display().to_string();
        }
        SourceChange::DataHomePath => {
            let previous = PathBuf::from(&binding.data_home);
            let next = previous.with_file_name(OTHER_HOME);
            let relative = Path::new(&binding.evidence_path)
                .strip_prefix(&previous)
                .unwrap();
            let evidence = next.join(relative);
            std::fs::rename(previous, &next).unwrap();
            binding.data_home = next.display().to_string();
            binding.evidence_path = evidence.display().to_string();
        }
        SourceChange::TranscriptObject => {
            let path = Path::new(&binding.evidence_path);
            let bytes = std::fs::read(path).unwrap();
            std::fs::rename(path, path.with_extension(RETAINED_OBJECT)).unwrap();
            std::fs::write(path, bytes).unwrap();
        }
        SourceChange::DirectoryObject => {
            let path = Path::new(&binding.cwd);
            std::fs::rename(path, path.with_file_name(RETAINED_OBJECT)).unwrap();
            std::fs::create_dir(path).unwrap();
        }
        SourceChange::DataHomeObject => {
            let home = Path::new(&binding.data_home);
            let retained = home.with_file_name(RETAINED_OBJECT);
            std::fs::rename(home, &retained).unwrap();
            std::fs::create_dir(home).unwrap();
            let evidence = crate::codex::constants::EVIDENCE_DIR;
            std::fs::rename(retained.join(evidence), home.join(evidence)).unwrap();
        }
    }
}

pub(super) fn rollout(binding: &AgentBinding) -> PathBuf {
    let constants = crate::codex::constants::ROLLOUT_FILE_PREFIX;
    let path = Path::new(&binding.data_home)
        .join(crate::codex::constants::EVIDENCE_DIR)
        .join(ROLLOUT_DATE)
        .join(format!(
            "{constants}{ROLLOUT_STAMP}-{}.{}",
            binding.provider_session_id,
            crate::constants::JSONL_EXTENSION
        ));
    let value = serde_json::json!({
        (crate::codex::constants::RECORD_TYPE_KEY): crate::codex::constants::SESSION_META_TYPE,
        (crate::codex::constants::PAYLOAD_KEY): {
            (crate::codex::constants::SESSION_ID_KEY): binding.provider_session_id,
            (crate::codex::constants::CWD_KEY): binding.cwd,
            (crate::codex::constants::SOURCE_KEY): crate::codex::constants::CLI_SOURCE,
        }
    });
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(&path, format!("{value}\n")).unwrap();
    path
}

pub(super) fn publish(fixture: &Fixture, authority: &CodexBindingAuthority) -> anyhow::Result<()> {
    fixture
        .store
        .codex_authority_store()?
        .publish_in_namespace_with_commit_fence(
            &fixture.namespace,
            fixture.store.path(),
            authority,
            || Ok(()),
        )
}

#[derive(Debug, Eq, PartialEq)]
pub(super) struct Stored {
    authority: (CodexBindingAuthority, crate::paths::identity::FileIdentity),
    projection: (AgentBinding, crate::paths::identity::FileIdentity),
}

pub(super) fn stored(fixture: &Fixture) -> Stored {
    let authority = fixture.store.codex_authority_store().unwrap();
    Stored {
        authority: fixture
            .namespace
            .read_json_with_identity(authority.path())
            .unwrap(),
        projection: fixture
            .namespace
            .read_json_with_identity(fixture.store.path())
            .unwrap(),
    }
}

pub(super) fn fixture() -> Fixture {
    let fixture = Fixture::new(AgentKind::Codex);
    fixture.repair().unwrap().unwrap();
    fixture
}
