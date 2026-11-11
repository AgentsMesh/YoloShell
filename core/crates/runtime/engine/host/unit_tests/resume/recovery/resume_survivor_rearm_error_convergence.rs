use crate::host::tests::*;
const RESUME_REARM_PATH_FILL: &str = "x";

#[derive(Clone, Copy)]
enum SurvivorRearmFault {
    DurableManifest,
    SurvivorEvidence,
    RearmEvidence,
}

#[tokio::test]
async fn resume_survivor_rearm_errors_fail_closed_with_durable_state() {
    for fault in [
        SurvivorRearmFault::DurableManifest,
        SurvivorRearmFault::SurvivorEvidence,
        SurvivorRearmFault::RearmEvidence,
    ] {
        assert_survivor_rearm_fault(fault).await;
    }
}

async fn assert_survivor_rearm_fault(fault: SurvivorRearmFault) {
    let directory = tempdir().unwrap();
    let host = unspawnable_resume_host(&directory);
    let (mut manifest, transaction, _) = dead_survivor_rearm_fixture(&host, directory.path());
    let expected = match fault {
        SurvivorRearmFault::DurableManifest => {
            corrupt_resume_json(&host.paths.manifest(manifest.terminal_id));
            transaction.clone()
        }
        SurvivorRearmFault::SurvivorEvidence => {
            corrupt_resume_json(&host.paths.runtime_record(manifest.terminal_id));
            transaction.clone()
        }
        SurvivorRearmFault::RearmEvidence => {
            let foreign =
                recreate_survivor_transaction(&host, &manifest, transaction.previous_runtime_epoch);
            host.publish_resume_transaction(&foreign).unwrap();
            foreign
        }
    };

    let corrupt = matches!(fault, SurvivorRearmFault::DurableManifest)
        .then(|| resume_manifest_identity(&host, manifest.terminal_id));
    let result = host
        .rearm_dead_resume_terminal(&mut manifest, &transaction)
        .await;

    match corrupt {
        Some(identity) => {
            assert_resume_failure_retains_corruption(&host, &expected, identity, result)
        }
        None => assert_resume_failure_retains(&host, &expected, result),
    }
}

#[test]
fn resume_survivor_rearm_plan_reports_launch_and_transaction_construction_errors() {
    let directory = tempdir().unwrap();
    let mut host = Host::open(HostConfig::new(directory.path())).unwrap();
    let manifest = fixture_manifest(directory.path());
    let transaction = recreate_survivor_transaction(&host, &manifest, Uuid::new_v4());
    let long_root = directory
        .path()
        .join(RESUME_REARM_PATH_FILL.repeat(crate::paths::constants::UNIX_SOCKET_PATH_MAX_BYTES));
    host.paths = Paths::new_with_socket_root(directory.path(), long_root);
    assert!(
        host.prepare_terminal_rearm(&manifest, &transaction, Uuid::new_v4())
            .is_err()
    );

    let second_directory = tempdir().unwrap();
    let host = Host::open(HostConfig::new(second_directory.path())).unwrap();
    let manifest = fixture_manifest(second_directory.path());
    let mut invalid = recreate_survivor_transaction(&host, &manifest, Uuid::new_v4());
    invalid.target = ResumeTarget::Shell {
        binding: ResumeBindingDisposition::Clear {
            operation_id: Uuid::nil(),
            previous: None,
            codex_authority: None,
        },
    };
    assert!(
        host.prepare_terminal_rearm(&manifest, &invalid, Uuid::new_v4())
            .is_err()
    );
}
