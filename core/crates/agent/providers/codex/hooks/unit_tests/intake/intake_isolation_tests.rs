use super::intake_test_support::*;
use crate::codex_hooks::CodexHookIntake;
use uuid::Uuid;

#[test]
fn intake_routes_are_isolated_by_runtime_epoch_and_invocation() {
    let root = tempfile::tempdir().unwrap();
    let paths = crate::paths::Paths::new(root.path().join(HOME_DIRECTORY));
    let session = Uuid::new_v4();
    let epoch = Uuid::new_v4();
    let invocation = Uuid::new_v4();
    let routes = [
        identity(session, epoch, invocation),
        identity(session, Uuid::new_v4(), invocation),
        identity(session, epoch, Uuid::new_v4()),
    ];
    let directories: Vec<_> = routes
        .iter()
        .map(|owner| artifacts(&paths, owner).intake_directory())
        .collect();
    assert_ne!(directories[0], directories[1]);
    assert_ne!(directories[0], directories[2]);

    for (owner, directory) in routes.iter().zip(&directories) {
        CodexHookIntake::for_identity(directory.clone(), owner)
            .unwrap()
            .persist(envelope(owner))
            .unwrap();
    }
    for (owner, directory) in routes.iter().zip(&directories) {
        let scan = CodexHookIntake::for_identity(directory.clone(), owner)
            .unwrap()
            .scan()
            .unwrap();
        assert_eq!(scan.occurrences.len(), 1);
        assert_eq!(&scan.occurrences[0].envelope().identity, owner);
    }
}
