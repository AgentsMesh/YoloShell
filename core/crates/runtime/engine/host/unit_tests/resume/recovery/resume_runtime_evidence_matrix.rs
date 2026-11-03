use crate::host::tests::*;
use super::*;

const RESUME_EVIDENCE_SCHEMA_OFFSET: u16 = 1;
const RESUME_EVIDENCE_GENERATION_OFFSET: u64 = 1;
pub(super) const RESUME_EVIDENCE_MISMATCH: &str = "resume-evidence-mismatch";
const RESUME_EVIDENCE_ARGUMENT: &str = "resume-evidence-mismatch-argument";

#[test]
fn previous_resume_runtime_requires_matching_identity_and_resolved_processes() {
    let directory = tempdir().unwrap();
    let mut host = Host::open(HostConfig::new(directory.path())).unwrap();
    host.boot_id = RESUME_TEST_BOOT_ID.into();
    let manifest = fixture_manifest(directory.path());
    let runtime = dead_resume_runtime(&host, &manifest);
    host.validate_previous_resume_runtime_core(&manifest, &runtime)
        .unwrap();
    host.validate_previous_resume_runtime(&manifest, &runtime)
        .unwrap();

    let mutations: [fn(&mut RuntimeRecord); 5] = [
        |value| value.schema_version += RESUME_EVIDENCE_SCHEMA_OFFSET,
        |value| value.terminal_id = Uuid::nil(),
        |value| value.runtime_epoch = Uuid::nil(),
        |value| value.worker_start_time = 0,
        |value| value.child_start_time = 0,
    ];
    for mutate in mutations {
        let mut candidate = runtime.clone();
        mutate(&mut candidate);
        assert!(
            host.validate_previous_resume_runtime_core(&manifest, &candidate)
                .is_err()
        );
        assert!(
            host.validate_previous_resume_runtime(&manifest, &candidate)
                .is_err()
        );
    }

    let mut previous_boot = runtime.clone();
    previous_boot.boot_id = RESUME_TEST_PREVIOUS_BOOT_ID.into();
    previous_boot.worker_pid = std::process::id();
    previous_boot.worker_start_time =
        process::process_start_time(previous_boot.worker_pid).unwrap();
    previous_boot.child_pid = unsafe { libc::getppid() as u32 };
    previous_boot.child_start_time = process::process_start_time(previous_boot.child_pid).unwrap();
    host.validate_previous_resume_runtime_core(&manifest, &previous_boot)
        .unwrap();
    host.validate_previous_resume_runtime(&manifest, &previous_boot)
        .unwrap();

    previous_boot.boot_id = host.boot_id.clone();
    assert!(
        host.validate_previous_resume_runtime(&manifest, &previous_boot)
            .is_err()
    );
    host.validate_previous_resume_runtime_core(&manifest, &previous_boot)
        .unwrap();
}

#[test]
fn planned_resume_runtime_rejects_every_transaction_field_mutation() {
    let directory = tempdir().unwrap();
    let mut host = Host::open(HostConfig::new(directory.path())).unwrap();
    host.boot_id = RESUME_TEST_BOOT_ID.into();
    let manifest = fixture_manifest(directory.path());
    let launch = shell_resume_launch(&host, &manifest);
    let transaction = shell_resume_transaction(Uuid::new_v4(), &launch);
    let runtime = planned_resume_runtime(
        &host,
        &transaction,
        RESUME_TEST_DEAD_WORKER_PID,
        RESUME_TEST_PROCESS_START_TIME,
        RESUME_TEST_DEAD_CHILD_PID,
        RESUME_TEST_PROCESS_START_TIME,
        BootstrapDelivery::NotRequired,
    );
    host.validate_planned_runtime_core(&manifest, &transaction, &runtime)
        .unwrap();

    let mutations: [fn(&mut RuntimeRecord); 18] = [
        |value| value.schema_version += RESUME_EVIDENCE_SCHEMA_OFFSET,
        |value| value.terminal_id = Uuid::nil(),
        |value| value.runtime_epoch = Uuid::nil(),
        |value| value.worker_id = Uuid::nil(),
        |value| value.socket_path.push(RESUME_EVIDENCE_MISMATCH),
        |value| value.auth_token.clear(),
        |value| value.command.push_str(RESUME_EVIDENCE_MISMATCH),
        |value| value.args.push(RESUME_EVIDENCE_ARGUMENT.into()),
        |value| value.cwd.push(RESUME_EVIDENCE_MISMATCH),
        |value| value.cols = 0,
        |value| value.rows = 0,
        |value| value.started_at_ms = 0,
        |value| value.controller_generation = 0,
        |value| value.worker_pid = 0,
        |value| value.child_pid = 0,
        |value| value.child_pid = value.worker_pid,
        |value| value.worker_start_time = 0,
        |value| value.child_start_time = 0,
    ];
    for mutate in mutations {
        let mut candidate = runtime.clone();
        mutate(&mut candidate);
        assert!(
            host.validate_planned_runtime_core(&manifest, &transaction, &candidate)
                .is_err()
        );
    }

    let mut future_generation = runtime;
    future_generation.controller_generation =
        host.controller_generation + RESUME_EVIDENCE_GENERATION_OFFSET;
    assert!(
        host.validate_planned_runtime_core(&manifest, &transaction, &future_generation)
            .is_err()
    );
}

#[test]
fn planned_committed_and_delivery_evidence_enforce_distinct_semantics() {
    let directory = tempdir().unwrap();
    let mut host = Host::open(HostConfig::new(directory.path())).unwrap();
    host.boot_id = RESUME_TEST_BOOT_ID.into();
    let manifest = fixture_manifest(directory.path());
    let launch = shell_resume_launch(&host, &manifest);
    let shell = shell_resume_transaction(Uuid::new_v4(), &launch);
    let mut runtime = planned_resume_runtime(
        &host,
        &shell,
        RESUME_TEST_DEAD_WORKER_PID,
        RESUME_TEST_PROCESS_START_TIME,
        RESUME_TEST_DEAD_CHILD_PID,
        RESUME_TEST_PROCESS_START_TIME,
        BootstrapDelivery::NotRequired,
    );
    host.validate_planned_runtime(&manifest, &shell, &runtime)
        .unwrap();
    host.validate_committed_resume_runtime(&manifest, &shell, &runtime)
        .unwrap();
    host.validate_resume_delivery(&shell, &runtime).unwrap();

    runtime.cols = runtime.cols.saturating_add(1);
    runtime.rows = runtime.rows.saturating_add(1);
    host.validate_planned_runtime(&manifest, &shell, &runtime)
        .unwrap();
    host.validate_committed_resume_runtime(&manifest, &shell, &runtime)
        .unwrap();

    runtime.boot_id = RESUME_TEST_PREVIOUS_BOOT_ID.into();
    assert!(
        host.validate_planned_runtime(&manifest, &shell, &runtime)
            .is_err()
    );
    host.validate_committed_resume_runtime(&manifest, &shell, &runtime)
        .unwrap();
    runtime.boot_id.clear();
    assert!(
        host.validate_committed_resume_runtime(&manifest, &shell, &runtime)
            .is_err()
    );

    let (_, _, provider) = provider_resume_fixture(&host, &manifest);
    let mut provider_runtime = planned_resume_runtime(
        &host,
        &provider,
        RESUME_TEST_DEAD_WORKER_PID,
        RESUME_TEST_PROCESS_START_TIME,
        RESUME_TEST_DEAD_CHILD_PID,
        RESUME_TEST_PROCESS_START_TIME,
        BootstrapDelivery::Delivered,
    );
    host.validate_resume_delivery(&provider, &provider_runtime)
        .unwrap();
    provider_runtime.bootstrap_delivery = BootstrapDelivery::Pending;
    assert!(
        host.validate_resume_delivery(&provider, &provider_runtime)
            .is_err()
    );
}
