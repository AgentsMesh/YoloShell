use crate::worker::tests::*;
#[cfg(target_os = "macos")]
pub(super) fn mark_finalizing(fixture: &Fixture) {
    fixture.state.child_waited.store(true, Ordering::Release);
    fixture.state.commit_output(FINALIZING_OUTPUT).unwrap();
    assert_eq!(fixture.state.status().status, FINALIZING_EXPECTED_STATUS);
}

#[cfg(target_os = "macos")]
pub(super) async fn adopt_finalizing(
    fixture: &Fixture,
) -> (
    SuccessorAdoptionWire,
    crate::pty_custody::CustodyOffer,
    tokio::task::JoinHandle<Result<()>>,
) {
    adopt_finalizing_with_wire(fixture, SuccessorAdoptionWire::install(fixture)).await
}

#[cfg(target_os = "macos")]
async fn adopt_finalizing_after_successor_loss(
    fixture: &Fixture,
) -> (
    SuccessorAdoptionWire,
    crate::pty_custody::CustodyOffer,
    tokio::task::JoinHandle<Result<()>>,
) {
    adopt_finalizing_with_wire(
        fixture,
        SuccessorAdoptionWire::install_after_successor_loss(fixture),
    )
    .await
}

#[cfg(target_os = "macos")]
async fn adopt_finalizing_with_wire(
    fixture: &Fixture,
    mut wire: SuccessorAdoptionWire,
) -> (
    SuccessorAdoptionWire,
    crate::pty_custody::CustodyOffer,
    tokio::task::JoinHandle<Result<()>>,
) {
    let task = wire.spawn(Arc::clone(&fixture.state), ADOPTION_TEST_TIMEOUT);
    let offer = match wire.receive_offer_result().await {
        Ok(offer) => offer,
        Err(error) => {
            let task_result = task.await.unwrap();
            panic!("finalizing custody handshake failed: {error:#}; task={task_result:?}");
        }
    };
    assert_eq!(
        offer.profile,
        crate::pty_custody::CustodyProfile::ActiveTerminalReadOnly
    );
    wire.receive_commit(&offer).await;
    wire.send_ready(&offer).await;
    (wire, offer, task)
}

#[cfg(target_os = "macos")]
#[tokio::test(flavor = "current_thread")]
async fn finalizing_successor_is_read_only_and_reports_status() {
    let fixture = Fixture::new();
    mark_finalizing(&fixture);
    let (mut wire, offer, adoption) = adopt_finalizing(&fixture).await;
    let mut peer = wire.take_peer();
    receive_exact_worker_activation(&mut peer, &offer, wire.request.controller_generation).await;

    let status = dispatch_test(&fixture.state, WORKER_STATUS_METHOD, &())
        .await
        .unwrap()
        .decode_payload::<RuntimeView>()
        .unwrap();
    assert_eq!(status.status, FINALIZING_EXPECTED_STATUS);
    assert_eq!(status.exit, None);
    assert_eq!(
        fixture
            .state
            .custody_profile(ControllerGeneration(wire.request.controller_generation)),
        Some(crate::pty_custody::CustodyProfile::ActiveTerminalReadOnly)
    );
    drop(peer);
    await_adoption_failure(adoption).await;
}

#[cfg(target_os = "macos")]
#[tokio::test(flavor = "current_thread")]
async fn finalizing_rejects_reads_and_all_runtime_mutations_until_exit_is_durable() {
    let fixture = Fixture::new();
    mark_finalizing(&fixture);
    let (mut wire, offer, adoption) = adopt_finalizing(&fixture).await;
    let mut peer = wire.take_peer();
    receive_exact_worker_activation(&mut peer, &offer, wire.request.controller_generation).await;
    assert_not_running_finalizing(&fixture).await;
    drop(peer);
    await_adoption_failure(adoption).await;
}

#[cfg(target_os = "macos")]
#[tokio::test(flavor = "current_thread")]
async fn durable_exit_opens_snapshot_and_attach_but_successor_loss_is_recoverable() {
    let fixture = Fixture::new();
    mark_finalizing(&fixture);
    let (mut wire, offer, adoption) = adopt_finalizing(&fixture).await;
    let mut peer = wire.take_peer();
    receive_exact_worker_activation(&mut peer, &offer, wire.request.controller_generation).await;
    let exit = ExitRecord {
        runtime_epoch: fixture.state.runtime_epoch(),
        code: Some(FINALIZING_EXIT_CODE),
        signal: FINALIZING_NO_SIGNAL,
        reason: FINALIZING_EXIT_REASON,
        observed_at_ms: now_ms(),
        last_output_sequence: fixture.state.journal.lock().unwrap().last_sequence(),
    };
    fixture
        .state
        .namespace
        .write_json(
            &fixture.state.paths.exit_record(fixture.terminal_id()),
            &exit,
        )
        .unwrap();
    *lock_recover(&fixture.state.exit) = Some(exit);
    assert!(
        dispatch_test(
            &fixture.state,
            method::GET_SNAPSHOT,
            &SnapshotRequest {
                terminal_id: fixture.terminal_id(),
                runtime_epoch: fixture.state.runtime_epoch(),
            }
        )
        .await
        .unwrap()
        .ok
    );
    let (mut client, task) = authenticated_worker_connection(
        &fixture,
        fixture.state.controller_generation.load(Ordering::Acquire),
    )
    .await;
    let attach = RequestEnvelope::new(
        Uuid::new_v4().to_string(),
        method::ATTACH_SESSION,
        to_value(&AttachRequest {
            terminal_id: fixture.terminal_id(),
            after_seq: FINALIZING_ATTACH_AFTER_SEQUENCE,
        })
        .unwrap(),
    );
    assert!(client.exchange(&attach).await.unwrap().ok);
    let event = client.next_event().await.unwrap();
    assert_eq!(event.event, event::SESSION_EXITED);
    // The completed subscription does not end its persistent Cap'n Proto connection.
    drop(client);
    tokio::time::timeout(ADOPTION_TEST_TIMEOUT, task)
        .await
        .expect(FINALIZING_ATTACHMENT_CLOSE_DETAIL)
        .unwrap()
        .unwrap();
    drop(peer);
    await_adoption_failure(adoption).await;

    let (mut retry_wire, retry_offer, retry_adoption) =
        adopt_finalizing_after_successor_loss(&fixture).await;
    let mut retry_peer = retry_wire.take_peer();
    receive_exact_worker_activation(
        &mut retry_peer,
        &retry_offer,
        retry_wire.request.controller_generation,
    )
    .await;
    drop(retry_peer);
    await_adoption_failure(retry_adoption).await;
}
