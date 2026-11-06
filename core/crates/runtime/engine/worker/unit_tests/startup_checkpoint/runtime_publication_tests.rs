use super::authority_test_support::*;
use super::retirement_test_support::runtime_for;
use super::*;

const CHANGED_ARGUMENT: &str = "changed-argument";
const CHANGED_AUTH_TOKEN: &str = "changed-auth-token";
const CHANGED_COMMAND: &str = "/changed/command";
const CHANGED_CWD: &str = "/changed/cwd";
const FIELD_OFFSET: u64 = 1;

#[test]
fn initial_publication_requires_every_runtime_launch_binding() {
    let mutations: [fn(&mut WorkerLaunchSpec); 12] = [
        |value| value.schema_version += u16::try_from(FIELD_OFFSET).unwrap(),
        |value| value.terminal_id = Uuid::new_v4(),
        |value| value.runtime_epoch = Uuid::new_v4(),
        |value| value.worker_id = Uuid::new_v4(),
        |value| value.controller_generation += FIELD_OFFSET,
        |value| value.socket_path.push(CHANGED_ARGUMENT),
        |value| value.auth_token = CHANGED_AUTH_TOKEN.into(),
        |value| value.command = CHANGED_COMMAND.into(),
        |value| value.args.push(CHANGED_ARGUMENT.into()),
        |value| value.cwd = CHANGED_CWD.into(),
        |value| value.cols += u16::try_from(FIELD_OFFSET).unwrap(),
        |value| value.rows += u16::try_from(FIELD_OFFSET).unwrap(),
    ];
    for mutate in mutations {
        let mut state = AuthorityFixture::new();
        let before = state.runtime_bytes();
        let mut changed = state.successor.clone();
        mutate(&mut changed);
        state.persist_launch(&changed);
        assert!(state.publish().is_err());
        assert_eq!(state.runtime_bytes(), before);
    }
}

#[test]
fn exact_launch_creates_absent_runtime_but_update_api_cannot_recreate_it() {
    let fixture = CheckpointFixture::new();
    let worker = fixture.worker;
    let child = fixture.child();
    let mut record = runtime_for(&fixture, worker, child);
    record.bootstrap_delivery = crate::model::BootstrapDelivery::Pending;
    persist_manifest(&fixture, &record);
    let mut checkpoint = fixture.begin();
    advance_to_runtime_record(&fixture, &mut checkpoint, child);

    crate::worker::runtime_record_publication::publish_initial_with_test_hook(
        &fixture.namespace,
        &fixture.paths,
        &record,
        &mut checkpoint,
        || Ok(()),
    )
    .unwrap();
    assert_eq!(read_runtime(&fixture), record);

    fixture
        .namespace
        .remove_file(&fixture.paths.runtime_record(record.terminal_id))
        .unwrap();
    assert!(
        crate::worker::publish_runtime_record(&fixture.namespace, &fixture.paths, &record).is_err()
    );
    assert!(!fixture.paths.runtime_record(record.terminal_id).exists());
}

#[test]
fn replacing_identical_launch_identity_is_rejected() {
    let mut state = AuthorityFixture::new();
    let before = state.runtime_bytes();
    let launch = state.successor.clone();
    state.persist_launch(&launch);
    assert!(state.publish().is_err());
    assert_eq!(state.runtime_bytes(), before);
}

#[test]
fn launch_and_manifest_changes_at_cas_boundary_are_rejected() {
    let mut launch_state = AuthorityFixture::new();
    let launch = launch_state.successor.clone();
    let launch_path = launch_state.fixture.paths.worker_launch(launch.terminal_id);
    assert!(
        publish_with(&mut launch_state, || {
            crate::paths::atomic_write_json(&launch_path, &launch)
        })
        .is_err()
    );

    for archive in [false, true] {
        let mut manifest_state = AuthorityFixture::new();
        let mut manifest = manifest_state.manifest();
        manifest.archived = archive;
        let path = manifest_state.fixture.paths.manifest(manifest.terminal_id);
        assert!(
            publish_with(&mut manifest_state, || {
                crate::paths::atomic_write_json(&path, &manifest)
            })
            .is_err()
        );
    }
}

fn publish_with<F>(state: &mut AuthorityFixture, hook: F) -> anyhow::Result<()>
where
    F: FnOnce() -> anyhow::Result<()>,
{
    crate::worker::runtime_record_publication::publish_initial_with_test_hook(
        &state.fixture.namespace,
        &state.fixture.paths,
        &state.replacement,
        &mut state.checkpoint,
        hook,
    )
}

fn read_runtime(fixture: &CheckpointFixture) -> crate::model::RuntimeRecord {
    crate::paths::read_json(&fixture.paths.runtime_record(fixture.launch.terminal_id)).unwrap()
}
