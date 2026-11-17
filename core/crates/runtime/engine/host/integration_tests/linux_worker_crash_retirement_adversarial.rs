use super::*;

#[derive(Debug)]
enum EvidenceTarget {
    Runtime,
    Custody,
    Containment,
    Journal,
    Cursor,
}

#[tokio::test]
async fn linux_crash_retirement_evidence_rejects_same_value_file_substitution() {
    for target in [
        EvidenceTarget::Runtime,
        EvidenceTarget::Custody,
        EvidenceTarget::Containment,
        EvidenceTarget::Journal,
        EvidenceTarget::Cursor,
    ] {
        let fixture = Fixture::new();
        fixture.settle().await;
        let evidence = fixture
            .host
            .secondary_custody
            .retirement_evidence(
                &fixture.host,
                &CustodyRuntimeIdentity::from_record(&fixture.runtime),
            )
            .unwrap();
        let journal = fixture.host.paths.journal(fixture.runtime.terminal_id);
        let path = match target {
            EvidenceTarget::Runtime => fixture
                .host
                .paths
                .runtime_record(fixture.runtime.terminal_id),
            EvidenceTarget::Custody => fixture.host.paths.pty_custody(fixture.runtime.terminal_id),
            EvidenceTarget::Containment => fixture.containment_path(),
            EvidenceTarget::Journal => journal,
            EvidenceTarget::Cursor => crate::journal::retained_cursor_path_for_test(&journal),
        };
        replace_same_value(&path).unwrap();

        assert!(evidence.verify(&fixture.host).is_err(), "{target:?}");
    }
}

#[tokio::test]
async fn linux_crash_exit_publication_rejects_receipt_or_containment_races() {
    for containment in [false, true] {
        let fixture = Fixture::new();
        fixture.settle().await;
        let path = if containment {
            fixture.containment_path()
        } else {
            fixture.host.paths.pty_custody(fixture.runtime.terminal_id)
        };
        let _hook =
            automatic_worker_crash_exit_test_hook::install(fixture.runtime.terminal_id, move || {
                replace_same_value(&path)
            });

        assert!(fixture.publish_exit().await.is_err());
        assert!(
            !fixture
                .host
                .paths
                .exit_record(fixture.runtime.terminal_id)
                .exists()
        );
    }
}

#[tokio::test]
async fn linux_matching_crash_exit_does_not_bypass_replaced_retirement_evidence() {
    let fixture = Fixture::new();
    fixture.publish_exit().await.unwrap();
    let exit = fixture.exit();
    let fixture = fixture.reopen();
    let mut containment: process::linux_containment::LinuxContainmentRecord = fixture
        .host
        .namespace
        .read_json(&fixture.containment_path())
        .unwrap();
    containment.runtime_epoch = Uuid::new_v4();
    fixture
        .host
        .namespace
        .write_json(&fixture.containment_path(), &containment)
        .unwrap();

    assert!(fixture.publish_exit().await.is_err());
    assert_eq!(fixture.exit(), exit);
}
