use super::*;

#[test]
fn serialized_evidence_has_only_the_sanitized_field_set() {
    let fixture = CheckpointFixture::new();
    fixture.begin();
    let bytes = std::fs::read(fixture.path()).unwrap();
    let object = serde_json::from_slice::<serde_json::Value>(&bytes)
        .unwrap()
        .as_object()
        .unwrap()
        .clone();
    let mut fields = object.keys().map(String::as_str).collect::<Vec<_>>();
    fields.sort_unstable();
    assert_eq!(fields, SANITIZED_FIELDS);
    for secret in [
        TEST_AUTH_TOKEN.as_bytes(),
        TEST_COMMAND.as_bytes(),
        TEST_ARGUMENT.as_bytes(),
        TEST_CWD.as_bytes(),
        TEST_ENVIRONMENT_KEY.as_bytes(),
        TEST_ENVIRONMENT_VALUE.as_bytes(),
        TEST_INITIAL_INPUT,
    ] {
        assert!(!bytes.windows(secret.len()).any(|window| window == secret));
    }
}

#[test]
fn malformed_unknown_and_oversized_records_are_rejected_and_preserved() {
    for bytes in [
        TEST_INVALID_CHECKPOINT_JSON.to_vec(),
        format!("{{\"{UNKNOWN_FIELD}\":true}}").into_bytes(),
    ] {
        let fixture = CheckpointFixture::new();
        fixture.write_bytes(&bytes, u64::try_from(bytes.len()).unwrap());
        assert!(io::read(fixture.namespace.file(&fixture.path()).unwrap(), || Ok(())).is_err());
        assert!(
            worker_startup_checkpoint_replay_disposition(&fixture.paths, &fixture.launch).is_err()
        );
        assert_eq!(std::fs::read(fixture.path()).unwrap(), bytes);
    }

    let fixture = CheckpointFixture::new();
    let evidence = fixture.begin().evidence;
    let mut oversized = serde_json::to_vec(&evidence).unwrap();
    oversized.resize(
        usize::try_from(crate::paths::constants::WORKER_STARTUP_CHECKPOINT_MAX_BYTES).unwrap()
            + OVERSIZE_SENTINEL_BYTES,
        b' ',
    );
    fixture.write_bytes(&oversized, u64::try_from(oversized.len()).unwrap());
    assert!(io::read(fixture.namespace.file(&fixture.path()).unwrap(), || Ok(())).is_err());
    assert_eq!(std::fs::read(fixture.path()).unwrap(), oversized);
}

#[test]
fn stale_identity_generation_and_progress_cannot_be_overwritten() {
    let mutations: [fn(&mut StartupCheckpointEvidence); 5] = [
        |value| value.runtime_epoch = Uuid::new_v4(),
        |value| value.worker_id = Uuid::new_v4(),
        |value| value.controller_generation += STALE_IDENTITY_OFFSET,
        |value| value.worker.start_time += STALE_IDENTITY_OFFSET,
        |value| value.stage = StartupStage::WorkerSocket,
    ];
    for mutate in mutations {
        let fixture = CheckpointFixture::new();
        let mut checkpoint = fixture.begin();
        let mut replacement = fixture.read();
        mutate(&mut replacement);
        crate::paths::atomic_write_json(&fixture.path(), &replacement).unwrap();
        assert!(
            checkpoint
                .enter(
                    &fixture.namespace,
                    &fixture.paths,
                    StartupStage::StartupContainment,
                )
                .is_err()
        );
        assert_eq!(fixture.read(), replacement);
    }
}

#[test]
fn exclusive_begin_preserves_every_preexisting_record() {
    let fixture = CheckpointFixture::new();
    let original = fixture.begin().evidence;
    let bytes = std::fs::read(fixture.path()).unwrap();
    assert!(
        ActiveStartupCheckpoint::begin(
            &fixture.namespace,
            &fixture.paths,
            &fixture.launch,
            fixture.worker,
        )
        .is_err()
    );
    assert_eq!(std::fs::read(fixture.path()).unwrap(), bytes);
    assert_eq!(fixture.read(), original);
}

#[test]
fn exclusive_begin_rejects_settled_evidence_outside_the_launch_fence() {
    let mutations: [fn(&mut StartupCheckpointEvidence); 4] = [
        |value| value.terminal_id = Uuid::new_v4(),
        |value| value.runtime_epoch = Uuid::new_v4(),
        |value| value.worker_id = Uuid::new_v4(),
        |value| value.controller_generation += STALE_IDENTITY_OFFSET,
    ];
    for mutate in mutations {
        let fixture = CheckpointFixture::new();
        let mut displaced = fixture.begin().evidence;
        displaced.status = StartupStatus::HostSettled;
        mutate(&mut displaced);
        crate::paths::atomic_write_json(&fixture.path(), &displaced).unwrap();

        assert!(
            ActiveStartupCheckpoint::begin(
                &fixture.namespace,
                &fixture.paths,
                &fixture.launch,
                fixture.worker,
            )
            .is_err()
        );
        assert_eq!(fixture.read(), displaced);
    }
}

#[test]
fn simultaneous_worker_and_child_must_have_distinct_process_ids() {
    let fixture = CheckpointFixture::new();
    super::retirement_test_support::activated_runtime(&fixture);
    let mut evidence = fixture.read();
    let child = evidence.child.as_mut().unwrap();
    child.pid = evidence.worker.pid;
    child.start_time = evidence.worker.start_time + STALE_IDENTITY_OFFSET;

    assert!(validation::validate(&evidence).is_err());
}

#[test]
fn atomic_cas_preserves_a_path_replacement() {
    let fixture = CheckpointFixture::new();
    let current = fixture.begin().evidence;
    let mut next = current.clone();
    next.stage = StartupStage::StartupContainment;
    next.phase = StartupPhase::Entered;
    let mut displaced = current.clone();
    displaced.controller_generation += STALE_IDENTITY_OFFSET;

    let result =
        io::replace_with_test_hook(&fixture.namespace, &fixture.paths, &current, &next, || {
            crate::paths::atomic_write_json(&fixture.path(), &displaced)
        });

    assert!(result.is_err());
    assert_eq!(fixture.read(), displaced);
}
