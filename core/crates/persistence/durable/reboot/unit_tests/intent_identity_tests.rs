use super::guard::SessionDurableMutationGuard;
use super::validation::test_fixture::Fixture;
use crate::namespace_lease::NamespaceLease;
use crate::paths::Paths;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};

const INTENT_REPLACEMENT_NAME: &str = "reboot-migration-replaced.json";
const NAMESPACE_LABEL: &str = "durable reboot intent identity test";

#[test]
fn publish_rejects_a_same_json_replacement_inode() {
    let fixture = Fixture::new();
    let paths = Paths::new(fixture._root.path());
    let terminal_id = fixture.intent.terminal_id;
    let session = paths.session_dir(terminal_id);
    let intent_path = paths.reboot_migration_intent(terminal_id);
    let retained = session.join(INTENT_REPLACEMENT_NAME);
    let namespace = NamespaceLease::capture(&[(NAMESPACE_LABEL, &session)]).unwrap();
    let guard = SessionDurableMutationGuard::acquire(&paths, &namespace, terminal_id).unwrap();
    let replace_path = intent_path.clone();
    let retained_path = retained.clone();
    let _hook = super::prepare_test_hook::install_after_intent_commit(move || {
        let bytes = fs::read(&replace_path)?;
        fs::rename(&replace_path, &retained_path)?;
        write_private(&replace_path, &bytes)
    });

    assert!(super::publish_intent(&paths, &guard, &fixture.intent).is_err());
    let (current, current_identity): (super::RebootMigrationIntent, _) =
        namespace.read_json_with_identity(&intent_path).unwrap();
    let (original, original_identity): (super::RebootMigrationIntent, _) =
        namespace.read_json_with_identity(&retained).unwrap();
    assert_eq!(current, fixture.intent);
    assert_eq!(original, fixture.intent);
    assert!(!current_identity.same_object(original_identity));
}

fn write_private(path: &std::path::Path, bytes: &[u8]) -> anyhow::Result<()> {
    let mut file = OpenOptions::new()
        .create_new(true)
        .write(true)
        .mode(crate::paths::constants::PRIVATE_FILE_MODE)
        .open(path)?;
    file.set_permissions(fs::Permissions::from_mode(
        crate::paths::constants::PRIVATE_FILE_MODE,
    ))?;
    file.write_all(bytes)?;
    file.sync_all()?;
    Ok(())
}
