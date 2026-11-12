use crate::host::tests::*;
use super::*;

const RESUME_LAUNCH_SCHEMA_OFFSET: u16 = 1;
const RESUME_LAUNCH_MISMATCH: &str = "resume-launch-mismatch";
const RESUME_LAUNCH_ARGUMENT: &str = "resume-launch-mismatch-argument";
const RESUME_LAUNCH_ENV_KEY: &str = "YOLO_RESUME_LAUNCH_MISMATCH";
const RESUME_LAUNCH_ENV_VALUE: &str = "set";
const RESUME_LAUNCH_INPUT: u8 = b'x';

#[test]
fn prepare_resume_launch_supports_both_targets_and_fences_existing_evidence() {
    let directory = tempdir().unwrap();
    let host = Host::open(HostConfig::new(directory.path())).unwrap();
    let manifest = fixture_manifest(directory.path());
    host.paths.ensure_session(manifest.terminal_id).unwrap();
    let shell = ResumeBinding::Shell(ResumeBindingDisposition::Preserve);
    let previous_runtime_epoch = Uuid::new_v4();
    let launch = host
        .prepare_resume_launch(
            &manifest,
            &shell,
            ResumeIntent::Explicit,
            previous_runtime_epoch,
        )
        .unwrap();
    atomic_write_json(&host.paths.worker_launch(manifest.terminal_id), &launch).unwrap();
    assert!(
        host.prepare_resume_launch(
            &manifest,
            &shell,
            ResumeIntent::Explicit,
            previous_runtime_epoch,
        )
        .is_err()
    );

    let provider_manifest = fixture_manifest(directory.path());
    host.paths
        .ensure_session(provider_manifest.terminal_id)
        .unwrap();
    let provider = ResumeBinding::Provider(host_binding(provider_manifest.terminal_id));
    let provider_launch = host.prepare_resume_launch(
        &provider_manifest,
        &provider,
        ResumeIntent::Explicit,
        Uuid::new_v4(),
    )
    .unwrap();
    assert_eq!(provider_launch.cwd, provider.launch_command(&provider_manifest).cwd);
    assert_ne!(provider_launch.cwd, provider_manifest.cwd);
}

#[test]
fn validate_resume_launch_rejects_identity_and_payload_mutations() {
    let directory = tempdir().unwrap();
    let host = Host::open(HostConfig::new(directory.path())).unwrap();
    let manifest = fixture_manifest(directory.path());
    let binding = ResumeBinding::Shell(ResumeBindingDisposition::Preserve);
    let launch = shell_resume_launch(&host, &manifest);
    let previous_runtime_epoch = Uuid::new_v4();
    host.validate_resume_launch(
        &manifest,
        &launch,
        &binding,
        ResumeIntent::Explicit,
        previous_runtime_epoch,
    )
    .unwrap();

    let mutations: [fn(&mut WorkerLaunchSpec); 15] = [
        |value| value.schema_version += RESUME_LAUNCH_SCHEMA_OFFSET,
        |value| value.terminal_id = Uuid::nil(),
        |value| value.runtime_epoch = Uuid::nil(),
        |value| value.worker_id = Uuid::nil(),
        |value| value.worker_id = value.runtime_epoch,
        |value| value.controller_generation = 0,
        |value| value.auth_token.clear(),
        |value| value.socket_path.push(RESUME_LAUNCH_MISMATCH),
        |value| value.command.push_str(RESUME_LAUNCH_MISMATCH),
        |value| value.args.push(RESUME_LAUNCH_ARGUMENT.into()),
        |value| value.cwd.push(RESUME_LAUNCH_MISMATCH),
        |value| {
            value
                .environment
                .insert(RESUME_LAUNCH_ENV_KEY.into(), RESUME_LAUNCH_ENV_VALUE.into());
        },
        |value| value.initial_input = Some(vec![RESUME_LAUNCH_INPUT]),
        |value| value.cols = value.cols.saturating_add(1),
        |value| value.rows = value.rows.saturating_add(1),
    ];
    for mutate in mutations {
        let mut candidate = launch.clone();
        mutate(&mut candidate);
        assert!(
            host.validate_resume_launch(
                &manifest,
                &candidate,
                &binding,
                ResumeIntent::Explicit,
                previous_runtime_epoch,
            )
            .is_err()
        );
    }

    let mut repeated_epoch = launch;
    repeated_epoch.runtime_epoch = previous_runtime_epoch;
    assert!(
        host.validate_resume_launch(
            &manifest,
            &repeated_epoch,
            &binding,
            ResumeIntent::Explicit,
            previous_runtime_epoch,
        )
        .is_err()
    );

    let (provider_binding, mut provider_launch, provider_transaction) =
        provider_resume_fixture(&host, &manifest);
    let provider_previous_runtime_epoch = provider_transaction.previous_runtime_epoch;
    host.validate_resume_launch(
        &manifest,
        &provider_launch,
        &provider_binding,
        ResumeIntent::Explicit,
        provider_previous_runtime_epoch,
    )
    .unwrap();
    provider_launch.initial_input = None;
    assert!(
        host.validate_resume_launch(
            &manifest,
            &provider_launch,
            &provider_binding,
            ResumeIntent::Explicit,
            provider_previous_runtime_epoch,
        )
        .is_err()
    );
}

#[test]
fn transaction_launch_validation_rejects_core_and_target_mutations() {
    let directory = tempdir().unwrap();
    let host = Host::open(HostConfig::new(directory.path())).unwrap();
    let manifest = fixture_manifest(directory.path());
    let launch = shell_resume_launch(&host, &manifest);
    let transaction = shell_resume_transaction(Uuid::new_v4(), &launch);
    host.validate_transaction_launch(&manifest, &transaction, &launch)
        .unwrap();

    let mutations: [fn(&mut WorkerLaunchSpec); 10] = [
        |value| value.schema_version += RESUME_LAUNCH_SCHEMA_OFFSET,
        |value| value.socket_path.push(RESUME_LAUNCH_MISMATCH),
        |value| value.command.push_str(RESUME_LAUNCH_MISMATCH),
        |value| value.args.push(RESUME_LAUNCH_ARGUMENT.into()),
        |value| value.cwd.push(RESUME_LAUNCH_MISMATCH),
        |value| value.cols = value.cols.saturating_add(1),
        |value| value.rows = value.rows.saturating_add(1),
        |value| value.controller_generation = 0,
        |value| value.auth_token.clear(),
        |value| value.initial_input = Some(vec![RESUME_LAUNCH_INPUT]),
    ];
    for mutate in mutations {
        let mut candidate = launch.clone();
        mutate(&mut candidate);
        let changed = retarget_resume_transaction(&transaction, candidate.clone());
        assert!(
            host.validate_transaction_launch(&manifest, &changed, &candidate)
                .is_err()
        );
    }

    let (_, mut provider_launch, provider) = provider_resume_fixture(&host, &manifest);
    host.validate_transaction_launch(&manifest, &provider, &provider_launch)
        .unwrap();
    provider_launch.initial_input = None;
    let changed = retarget_resume_transaction(&provider, provider_launch.clone());
    assert!(
        host.validate_transaction_launch(&manifest, &changed, &provider_launch)
            .is_err()
    );
}
