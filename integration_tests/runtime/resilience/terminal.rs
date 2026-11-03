use super::*;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn pty_input_output_resize_and_idempotent_input() {
    const INPUT: &[u8] = b"yolo-echo\n";
    const EXPECTED_OUTPUT: &[u8] = b"yolo-echo";
    let harness = Harness::start().await;
    let session = harness.create_cat("interactive").await;
    let id = session.manifest.terminal_id;
    let target = runtime_ref_from_view(&session);
    let attachment = harness.client().attach(id, 0).await.unwrap();
    let lease = harness
        .client()
        .acquire_writer(target, "window-a", None)
        .await
        .unwrap();
    harness
        .client()
        .resize(target, lease.generation, &lease.writer_capability, 132, 44)
        .await
        .unwrap();
    let input_id = Uuid::new_v4();
    let input = InputRequest {
        terminal_id: id,
        runtime_epoch: target.runtime_epoch,
        generation: lease.generation,
        writer_capability: lease.writer_capability.clone(),
        input_id,
        data: INPUT.to_vec(),
    };
    assert!(
        !harness
            .client()
            .input_with_id(&input)
            .await
            .unwrap()
            .duplicate
    );
    assert!(
        harness
            .client()
            .input_with_id(&input)
            .await
            .unwrap()
            .duplicate
    );
    let output = read_until(attachment, EXPECTED_OUTPUT).await;
    assert!(
        output
            .windows(EXPECTED_OUTPUT.len())
            .any(|value| value == EXPECTED_OUTPUT)
    );
    let runtime: RuntimeRecord = read_json(&harness.paths().runtime_record(id)).unwrap();
    assert_eq!((runtime.cols, runtime.rows), (132, 44));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn conflicting_writers_are_fenced_and_takeover_revokes_old_generation() {
    const FIRST_HOLDER: &str = "window-a";
    const FIRST_INSTANCE: &str = "main-a-v1";
    const SECOND_HOLDER: &str = "window-b";
    const SECOND_INSTANCE: &str = "main-b-v1";
    const REPLACEMENT_INSTANCE: &str = "main-b-v2";
    let harness = Harness::start().await;
    let session = harness.create_cat("writers").await;
    let id = session.manifest.terminal_id;
    let target = runtime_ref_from_view(&session);
    let mut attachment = harness.client().attach(id, 0).await.unwrap();
    let first_capability = WriterCapability::generate();
    let first = harness
        .client()
        .acquire_writer_for_instance(
            target,
            FIRST_HOLDER,
            Some(FIRST_INSTANCE.into()),
            first_capability,
            None,
        )
        .await
        .unwrap();
    let acquired = next_event_of_type(&mut attachment, event::WRITER_LEASE_ACQUIRED).await;
    let acquired: WriterLease = yolo_runtime::protocol::from_value(acquired.payload).unwrap();
    assert_eq!(acquired.generation, first.generation);
    assert_eq!(
        harness.client().get_writer_lease(target).await.unwrap(),
        Some(first.lease.clone())
    );
    let conflict = harness
        .client()
        .acquire_writer_for_instance(
            target,
            SECOND_HOLDER,
            Some(SECOND_INSTANCE.into()),
            WriterCapability::generate(),
            None,
        )
        .await
        .unwrap_err();
    assert!(
        matches!(conflict, ClientError::Remote { code, .. } if code == lease_error_code::HELD_BY_OTHER)
    );
    let second = harness
        .client()
        .take_over_writer_for_instance(
            target,
            SECOND_HOLDER,
            Some(SECOND_INSTANCE.into()),
            Some(first.generation),
            WriterCapability::generate(),
            None,
        )
        .await
        .unwrap();
    let revoked = next_event_of_type(&mut attachment, event::WRITER_LEASE_REVOKED).await;
    let revoked: LeaseEventPayload = yolo_runtime::protocol::from_value(revoked.payload).unwrap();
    assert_eq!(revoked.generation, first.generation);
    assert_eq!(revoked.reason, Some(LeaseReason::TakenOver));
    let acquired = next_event_of_type(&mut attachment, event::WRITER_LEASE_ACQUIRED).await;
    let acquired: WriterLease = yolo_runtime::protocol::from_value(acquired.payload).unwrap();
    assert_eq!(acquired.generation, second.generation);
    assert!(second.generation > first.generation);
    assert_eq!(
        harness.client().get_writer_lease(target).await.unwrap(),
        Some(second.lease.clone())
    );
    let stale = harness
        .client()
        .input(
            target,
            first.generation,
            &first.writer_capability,
            b"stale\n".to_vec(),
        )
        .await
        .unwrap_err();
    assert!(
        matches!(stale, ClientError::Remote { code, .. } if code == lease_error_code::STALE_GENERATION)
    );
    let stale_signal = harness
        .client()
        .signal(target, first.generation, &first.writer_capability, "INT")
        .await
        .unwrap_err();
    assert!(
        matches!(stale_signal, ClientError::Remote { code, .. } if code == lease_error_code::STALE_GENERATION)
    );
    harness
        .client()
        .input(
            target,
            second.generation,
            &second.writer_capability,
            b"current\n".to_vec(),
        )
        .await
        .unwrap();
    let replacement_capability = WriterCapability::generate();
    let replacement = harness
        .client()
        .take_over_writer_for_instance(
            target,
            SECOND_HOLDER,
            Some(REPLACEMENT_INSTANCE.into()),
            Some(second.generation),
            replacement_capability.clone(),
            None,
        )
        .await
        .unwrap();
    let revoked = next_event_of_type(&mut attachment, event::WRITER_LEASE_REVOKED).await;
    let revoked: LeaseEventPayload = yolo_runtime::protocol::from_value(revoked.payload).unwrap();
    assert_eq!(revoked.generation, second.generation);
    assert_eq!(revoked.reason, Some(LeaseReason::ControllerReplaced));
    assert!(replacement.generation > second.generation);
    let retry = harness
        .client()
        .acquire_writer_for_instance(
            target,
            SECOND_HOLDER,
            Some(REPLACEMENT_INSTANCE.into()),
            replacement_capability,
            None,
        )
        .await
        .unwrap();
    assert_eq!(retry.generation, replacement.generation);
    let fenced = harness
        .client()
        .renew_writer(&second, target, None)
        .await
        .unwrap_err();
    assert!(
        matches!(fenced, ClientError::Remote { code, .. } if code == lease_error_code::CONTROLLER_INSTANCE_MISMATCH)
    );
}
