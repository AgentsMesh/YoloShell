use crate::host::tests::*;
use super::*;

#[test]
fn valid_partial_catalog_from_interrupted_rebuild_is_rebuilt_again() {
    let directory = tempdir().unwrap();
    let first = persist_rebuild_manifest(directory.path());
    let second = persist_rebuild_manifest(directory.path());
    let paths = Paths::new(directory.path());
    let mut partial = Catalog::open(&paths.catalog()).unwrap();
    partial.insert(&first).unwrap();
    partial.integrity_check().unwrap();
    drop(partial);

    let host = Host::open(HostConfig::new(directory.path())).unwrap();

    let catalog = lock_recover(&host.catalog);
    assert_eq!(catalog.list(true).unwrap().len(), 2);
    assert!(catalog.get(first.terminal_id).unwrap().is_some());
    assert!(catalog.get(second.terminal_id).unwrap().is_some());
    drop(catalog);
    assert_quarantined_catalog(directory.path());
}

#[test]
fn complete_catalog_rebuild_with_unrestored_generation_recovers_worker_fence() {
    const DURABLE_GENERATION: u64 = 8;

    let directory = tempdir().unwrap();
    let original = Host::open(HostConfig::new(directory.path())).unwrap();
    let manifest = persist_rebuild_manifest(directory.path());
    let mut record = host_runtime_record(
        &original,
        manifest.terminal_id,
        directory.path().join("worker.sock"),
    );
    record.controller_generation = DURABLE_GENERATION;
    atomic_write_json(&original.paths.runtime_record(manifest.terminal_id), &record).unwrap();
    lock_recover(&original.catalog).insert(&manifest).unwrap();
    drop(original);

    let host = Host::open(HostConfig::new(directory.path())).unwrap();

    assert_eq!(host.controller_generation(), DURABLE_GENERATION + 1);
}

#[test]
fn row_rebuild_preserves_catalog_generation_without_runtime_evidence() {
    const CATALOG_GENERATION_FLOOR: u64 = 21;

    let directory = tempdir().unwrap();
    let original = Host::open(HostConfig::new(directory.path())).unwrap();
    let manifest = persist_rebuild_manifest(directory.path());
    {
        let mut catalog = lock_recover(&original.catalog);
        catalog.insert(&manifest).unwrap();
        catalog
            .restore_controller_generation_floor(CATALOG_GENERATION_FLOOR)
            .unwrap();
    }
    corrupt_catalog_manifest(&original, manifest.terminal_id);
    drop(original);

    let recovered = Host::open(HostConfig::new(directory.path())).unwrap();

    assert_eq!(
        recovered.controller_generation(),
        CATALOG_GENERATION_FLOOR + 1
    );
    assert_rebuilt_manifest(&recovered, &manifest);
}

#[test]
fn catalog_rebuild_uses_launch_generation_before_runtime_record_exists() {
    const LAUNCH_GENERATION: u64 = 13;
    const LAUNCH_TOKEN: &str = "launch-token";

    let directory = tempdir().unwrap();
    let original = Host::open(HostConfig::new(directory.path())).unwrap();
    let manifest = persist_rebuild_manifest(directory.path());
    let launch = WorkerLaunchSpec {
        schema_version: WORKER_LAUNCH_SCHEMA_VERSION,
        terminal_id: manifest.terminal_id,
        runtime_epoch: Uuid::new_v4(),
        worker_id: Uuid::new_v4(),
        socket_path: original.paths.worker_socket(manifest.terminal_id).unwrap(),
        auth_token: LAUNCH_TOKEN.into(),
        controller_generation: LAUNCH_GENERATION,
        command: manifest.command.clone(),
        args: manifest.args.clone(),
        cwd: manifest.cwd.clone(),
        environment: Default::default(),
        initial_input: None,
        cols: manifest.cols,
        rows: manifest.rows,
        context: None,
    };
    atomic_write_json(&original.paths.worker_launch(manifest.terminal_id), &launch).unwrap();
    assert!(!original.paths.runtime_record(manifest.terminal_id).exists());
    drop(original);
    std::fs::write(
        Paths::new(directory.path()).catalog(),
        CORRUPT_CATALOG_BYTES,
    )
    .unwrap();

    let host = Host::open(HostConfig::new(directory.path())).unwrap();

    assert_eq!(host.controller_generation(), LAUNCH_GENERATION + 1);
    assert_rebuilt_manifest(&host, &manifest);
}

#[test]
fn generation_above_sqlite_integer_range_increments_across_reopens() {
    const DURABLE_GENERATION: u64 = i64::MAX as u64 + 1;

    let directory = tempdir().unwrap();
    persist_runtime_generation(directory.path(), DURABLE_GENERATION);

    let first = Host::open(HostConfig::new(directory.path())).unwrap();
    assert_eq!(first.controller_generation(), DURABLE_GENERATION + 1);
    drop(first);
    let second = Host::open(HostConfig::new(directory.path())).unwrap();

    assert_eq!(second.controller_generation(), DURABLE_GENERATION + 2);
}

#[test]
fn maximum_durable_generation_fails_closed_without_wrapping() {
    let directory = tempdir().unwrap();
    persist_runtime_generation(directory.path(), u64::MAX);

    let error = Host::open(HostConfig::new(directory.path())).err().unwrap();
    let retry_error = Host::open(HostConfig::new(directory.path())).err().unwrap();

    for failure in [error, retry_error] {
        assert!(format!("{failure:#}").contains(CONTROLLER_GENERATION_EXHAUSTED_DETAIL));
    }
    let catalog = Catalog::open(&Paths::new(directory.path()).catalog()).unwrap();
    assert_eq!(catalog.controller_generation().unwrap(), u64::MAX);
}

fn persist_runtime_generation(home: &Path, generation: u64) {
    let original = Host::open(HostConfig::new(home)).unwrap();
    let manifest = persist_rebuild_manifest(home);
    let mut record = host_runtime_record(
        &original,
        manifest.terminal_id,
        home.join("generation-worker.sock"),
    );
    record.controller_generation = generation;
    atomic_write_json(&original.paths.runtime_record(manifest.terminal_id), &record).unwrap();
}
const CONTROLLER_GENERATION_EXHAUSTED_DETAIL: &str = "controller generation exhausted";
