const CUSTODY_SUCCESSOR_GENERATION: u64 = FIXTURE_CONTROLLER_GENERATION + 1;
const CUSTODY_RELEASE_TIMEOUT: Duration = Duration::from_secs(2);

fn test_custody_lease(
    fixture: &Fixture,
    identity: secondary_custody_authority::ReadyWorkerCustodyIdentity,
) -> (WorkerCustodyLease, UnixStream) {
    let (worker, peer) = UnixStream::pair().unwrap();
    let stream = crate::pty_custody::CustodyUnixStream::from_local(worker).unwrap();
    (
        WorkerCustodyLease::from_ready_for_test(
            stream,
            identity,
            crate::pty_custody::CustodyRuntimeIdentity::from_record(&lock_recover(
                &fixture.state.record,
            )),
        ),
        peer,
    )
}

#[cfg(target_os = "macos")]
const CUSTODY_TEST_CAPABILITY: &str = "custody-holder-indeterminate-test";
#[cfg(target_os = "macos")]
const CUSTODY_TEST_READER_HANDLE: u64 = 70_003;
#[cfg(target_os = "macos")]
const CUSTODY_SUCCESSOR_INCREMENT: u64 = 1;
#[cfg(target_os = "macos")]
const CUSTODY_TEST_WRITER_HANDLE: u64 = 70_004;

#[cfg(target_os = "macos")]
fn custody_source_for_test(
    record: &RuntimeRecord,
) -> (
    WorkerSecondaryCustodySource,
    portable_pty::PtyPair,
    crate::process::MacosProcessCapabilityChildWriter,
) {
    custody_source_with_transfer_for_test(
        record,
        Arc::new(startup_custody_transfer::StartupCustodyTransferFence::new()),
        None,
    )
}

#[cfg(target_os = "macos")]
fn custody_source_with_transfer_for_test(
    record: &RuntimeRecord,
    transfer: Arc<startup_custody_transfer::StartupCustodyTransferFence>,
    fault: Option<secondary_custody_source::InitialCustodyHandshakeBoundary>,
) -> (
    WorkerSecondaryCustodySource,
    portable_pty::PtyPair,
    crate::process::MacosProcessCapabilityChildWriter,
) {
    let pair = native_pty_system()
        .openpty(PtySize {
            rows: FIXTURE_TERMINAL_ROWS,
            cols: FIXTURE_TERMINAL_COLUMNS,
            pixel_width: FIXTURE_PIXEL_DIMENSION,
            pixel_height: FIXTURE_PIXEL_DIMENSION,
        })
        .unwrap();
    let mut proof = crate::process::PtySessionProof::capture_bound_for_test(
        pair.master.as_ref(),
        CUSTODY_TEST_CAPABILITY.to_owned(),
    )
    .unwrap();
    let (reader, writer) = crate::process::MacosProcessCapabilityIssuer::create()
        .unwrap()
        .into_staged_for_test()
        .unwrap();
    proof.install_process_capability_for_test(reader).unwrap();
    let runtime = crate::pty_custody::CustodyRuntimeIdentity::from_record(record);
    let _alias_release = proof.bind_secondary_custody_alias_for_test(&runtime).unwrap();
    let export = crate::process::MacosSecondaryCustodyExport::capture_for_test(&proof, &runtime).unwrap();
    let source = match fault {
        Some(fault) => WorkerSecondaryCustodySource::new_faulted_for_test(export, transfer, fault),
        None => WorkerSecondaryCustodySource::new(export, transfer),
    };
    (source, pair, writer)
}

#[cfg(target_os = "macos")]
fn armed_custody_for_test(
    record: &RuntimeRecord,
    holder: crate::pty_custody::CustodyHolderIdentity,
) -> crate::pty_custody::CustodyDurableRecord {
    let identity = secondary_custody_authority::ReadyWorkerCustodyIdentity::fresh_for_test(
        record.controller_generation,
    )
    .with_holder_for_test(holder);
    armed_custody_with_identity_for_test(record, identity)
}

#[cfg(target_os = "macos")]
fn armed_custody_with_identity_for_test(
    record: &RuntimeRecord,
    identity: secondary_custody_authority::ReadyWorkerCustodyIdentity,
) -> crate::pty_custody::CustodyDurableRecord {
    let runtime = crate::pty_custody::CustodyRuntimeIdentity::from_record(record);
    let pending = crate::pty_custody::CustodyDurableRecord::pending(runtime.clone());
    let request = crate::pty_custody::CustodyRequest::new(
        identity.lease_id(), runtime, identity.holder(), record.auth_token.clone(), identity.generation(),
    );
    let offer = crate::pty_custody::CustodyOffer {
        protocol_version: crate::pty_custody::PROTOCOL_VERSION,
        lease_id: identity.lease_id(),
        offer_id: identity.offer_id(),
        runtime: request.runtime.clone(),
        descriptor_set: crate::pty_custody::CustodyDescriptorSet::macos_process_capability(
            CUSTODY_TEST_READER_HANDLE, CUSTODY_TEST_WRITER_HANDLE,
        ),
        profile: identity.profile(),
        recovery_identity_sha256: identity.recovery_identity_sha256(),
    };
    crate::pty_custody::arm_fixture(&pending, &request, &offer).unwrap()
}
