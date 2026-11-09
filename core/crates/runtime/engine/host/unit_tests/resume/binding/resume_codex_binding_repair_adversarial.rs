use crate::host::tests::*;
use super::*;

const FOREIGN_PROCESS_START_INCREMENT: u64 = 1;

#[derive(Clone, Copy)]
enum UnsafeBindingPredecessor {
    Claude,
    CodexSession,
    ProviderProcess,
}

#[tokio::test]
async fn preserve_resume_does_not_repair_an_authority_from_another_runtime() {
    let (_directory, host, mut manifest, binding, store) = codex_repair_fixture();
    ProviderAuthorityPublisher::new()
        .publish(&host, &binding)
        .unwrap();
    host.namespace.remove_file(store.path()).unwrap();
    let runtime_path = host.paths.runtime_record(manifest.terminal_id);
    let mut replacement: RuntimeRecord = host.read_namespace_json(&runtime_path).unwrap();
    replacement.runtime_epoch = Uuid::new_v4();
    host.write_namespace_json(&runtime_path, &replacement)
        .unwrap();

    let resumed = host
        .binding_for_resume(
            &mut manifest,
            ResumeSelection::Preserve(ResumeIntent::Automatic),
        )
        .await;

    assert!(resumed.is_err());
    assert!(host.namespace.file_is_missing(store.path()).unwrap());
}

#[tokio::test]
async fn preserve_resume_never_overwrites_an_unsafe_binding_predecessor() {
    for damage in [
        UnsafeBindingPredecessor::Claude,
        UnsafeBindingPredecessor::CodexSession,
        UnsafeBindingPredecessor::ProviderProcess,
    ] {
        let (_directory, host, mut manifest, target, store) = codex_repair_fixture();
        ProviderAuthorityPublisher::new()
            .publish(&host, &target)
            .unwrap();
        let foreign = unsafe_binding_predecessor(&target, damage);
        host.write_namespace_json(store.path(), &foreign).unwrap();

        let resumed = host
            .binding_for_resume(
                &mut manifest,
                ResumeSelection::Preserve(ResumeIntent::Automatic),
            )
            .await;

        assert!(resumed.is_err());
        assert_eq!(
            store.read_raw_in_namespace(&host.namespace).unwrap(),
            Some(foreign)
        );
    }
}

fn unsafe_binding_predecessor(
    target: &AgentBinding,
    damage: UnsafeBindingPredecessor,
) -> AgentBinding {
    let mut foreign = target.clone();
    match damage {
        UnsafeBindingPredecessor::Claude => foreign.kind = crate::agent::AgentKind::ClaudeCode,
        UnsafeBindingPredecessor::CodexSession => {
            foreign.provider_session_id = Uuid::new_v4().hyphenated().to_string();
        }
        UnsafeBindingPredecessor::ProviderProcess => {
            foreign.process_start_time = foreign
                .process_start_time
                .and_then(|start| start.checked_add(FOREIGN_PROCESS_START_INCREMENT));
        }
    }
    foreign
}
