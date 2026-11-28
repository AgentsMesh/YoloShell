use super::*;
use crate::host::tests::*;

#[test]
fn agent_authority_must_match_the_observed_runtime_epoch() {
    let observed = Uuid::new_v4();
    let replacement = Uuid::new_v4();
    assert!(runtime_epochs_pair(Some(observed), Some(observed)));
    assert!(!runtime_epochs_pair(Some(observed), Some(replacement)));
    assert!(runtime_epochs_pair(None, Some(observed)));
    assert!(!runtime_epochs_pair(Some(observed), None));
}

#[test]
fn a_delayed_view_cannot_publish_a_binding_after_worker_replacement() {
    let directory = tempdir().unwrap();
    let host = Host::open(HostConfig::new(directory.path())).unwrap();
    let mut manifest = fixture_manifest(directory.path());
    let id = manifest.terminal_id;
    let binding = completion_provider_binding(&directory, id);
    manifest.cwd = binding.cwd.clone().into();
    let observed = host_runtime_record(&host, id, directory.path().join("worker.sock"));
    persist_host_session(&host, &manifest, &observed);
    AgentBindingStore::new(host.paths.agent_binding(id))
        .save(&binding)
        .unwrap();
    let read = || {
        host.load_agent_binding_with_observed_runtime(&manifest, Some(&observed))
            .unwrap()
    };
    assert!(read().0.is_some());
    let mut successor = observed.clone();
    successor.runtime_epoch = Uuid::new_v4();
    atomic_write_json(&host.paths.runtime_record(id), &successor).unwrap();
    assert_eq!(read(), (None, None));
}
