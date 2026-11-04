use crate::host::tests::*;
use super::*;

const CREATION_RETRY_GENERATION_INCREMENT: u64 = 1;
const CREATION_SCHEMA_INCREMENT: u16 = 1;

#[tokio::test]
async fn convergence_retried_creation_classifies_previous_launch_and_runtime_identity() {
    let (_directory, host, _, binding, planned, runtime) = pending_creation_fixture(false).await;
    let mut retry = binding.clone();
    retry.phase = CreationPhase::Launching;
    retry.launch_generation =
        INITIAL_CREATION_LAUNCH_GENERATION + CREATION_RETRY_GENERATION_INCREMENT;
    retry.launch = Some(planned.clone());

    let mut previous_launch = planned_worker_launch(&host, &retry, &planned);
    previous_launch.runtime_epoch = Uuid::new_v4();
    assert_eq!(
        classify_creation_launch(Some(&previous_launch), &retry, &planned, &host.paths),
        CreationEvidenceIdentity::Previous
    );

    let mut previous_runtime = runtime;
    previous_runtime.runtime_epoch = Uuid::new_v4();
    assert_eq!(
        classify_creation_runtime(Some(&previous_runtime), None, &retry, &planned, &host.paths,),
        CreationEvidenceIdentity::Previous
    );

    previous_launch.schema_version += CREATION_SCHEMA_INCREMENT;
    assert_eq!(
        classify_creation_launch(Some(&previous_launch), &retry, &planned, &host.paths),
        CreationEvidenceIdentity::Invalid
    );
    previous_runtime.schema_version += CREATION_SCHEMA_INCREMENT;
    assert_eq!(
        classify_creation_runtime(Some(&previous_runtime), None, &retry, &planned, &host.paths,),
        CreationEvidenceIdentity::Invalid
    );
}

#[tokio::test]
async fn convergence_creation_product_validation_reaches_each_identity_gate() {
    let (_directory, host, _, binding, _, runtime) = pending_creation_fixture(false).await;
    let launch: WorkerLaunchSpec =
        read_json(&host.paths.worker_launch(binding.result.terminal_id)).unwrap();
    assert!(launch_matches_product(&launch, &binding, &host.paths));
    assert!(runtime_matches_product(&runtime, &binding, &host.paths));

    for mutation in [
        CreationProductMutation::RuntimeEpoch,
        CreationProductMutation::Worker,
        CreationProductMutation::AuthToken,
    ] {
        let mut candidate = launch.clone();
        mutation.apply_launch(&mut candidate);
        assert!(!launch_matches_product(&candidate, &binding, &host.paths));

        let mut candidate = runtime.clone();
        mutation.apply_runtime(&mut candidate);
        assert!(!runtime_matches_product(&candidate, &binding, &host.paths));
    }
}

#[tokio::test]
async fn convergence_invalid_launch_is_reported_as_creation_identity_mismatch() {
    let (_directory, host, _, binding, planned, _) = pending_creation_fixture(false).await;
    let path = host.paths.worker_launch(binding.result.terminal_id);
    let mut launch: WorkerLaunchSpec = read_json(&path).unwrap();
    launch.schema_version += CREATION_SCHEMA_INCREMENT;
    atomic_write_json(&path, &launch).unwrap();

    assert_eq!(
        host.creation_runtime_evidence(&binding, &planned),
        CreationRuntimeEvidence::Mismatch
    );
}

#[tokio::test]
async fn convergence_one_live_process_keeps_failed_creation_evidence_active() {
    let (_directory, host, _, _, _, mut runtime) = pending_creation_fixture(false).await;
    runtime.child_pid = std::process::id();
    runtime.child_start_time = process::process_start_time(runtime.child_pid).unwrap();

    assert_eq!(
        host.creation_failure_resolution(&runtime),
        CreationFailureResolution::Active
    );
}

#[tokio::test]
async fn convergence_authenticated_creation_rechecks_delivery_and_record_equality() {
    for mutation in [
        AuthenticatedCreationMutation::Pending,
        AuthenticatedCreationMutation::Survivor,
    ] {
        let (_directory, host, _, binding, planned, mut runtime) =
            pending_creation_fixture(true).await;
        runtime.bootstrap_delivery = BootstrapDelivery::NotRequired;
        let path = host.paths.runtime_record(binding.result.terminal_id);
        atomic_write_json(&path, &runtime).unwrap();
        let listener = UnixListener::bind(&runtime.socket_path).unwrap();
        let mut replacement = runtime.clone();
        replacement.bootstrap_delivery = mutation.delivery();
        let server = tokio::spawn(serve_mutating_creation_auth(
            listener,
            runtime,
            path,
            replacement,
        ));

        let evidence = host
            .authenticated_creation_runtime_evidence(&binding, &planned)
            .await;
        server.await.unwrap();
        assert_eq!(evidence, mutation.expected());
    }
}

#[derive(Clone, Copy)]
enum CreationProductMutation {
    RuntimeEpoch,
    Worker,
    AuthToken,
}

impl CreationProductMutation {
    fn apply_launch(self, launch: &mut WorkerLaunchSpec) {
        match self {
            Self::RuntimeEpoch => launch.runtime_epoch = Uuid::nil(),
            Self::Worker => launch.worker_id = Uuid::nil(),
            Self::AuthToken => launch.auth_token.clear(),
        }
    }

    fn apply_runtime(self, runtime: &mut RuntimeRecord) {
        match self {
            Self::RuntimeEpoch => runtime.runtime_epoch = Uuid::nil(),
            Self::Worker => runtime.worker_id = Uuid::nil(),
            Self::AuthToken => runtime.auth_token.clear(),
        }
    }
}

#[derive(Clone, Copy)]
enum AuthenticatedCreationMutation {
    Pending,
    Survivor,
}

impl AuthenticatedCreationMutation {
    const fn delivery(self) -> BootstrapDelivery {
        match self {
            Self::Pending => BootstrapDelivery::Pending,
            Self::Survivor => BootstrapDelivery::Survivor,
        }
    }

    const fn expected(self) -> CreationRuntimeEvidence {
        match self {
            Self::Pending => CreationRuntimeEvidence::MatchingPending,
            Self::Survivor => CreationRuntimeEvidence::Mismatch,
        }
    }
}

async fn serve_mutating_creation_auth(
    listener: UnixListener,
    authenticated: RuntimeRecord,
    path: PathBuf,
    replacement: RuntimeRecord,
) {
    let socket_path = authenticated.socket_path.clone();
    let (mut stream, _) = listener.accept().await.unwrap();
    let request: RequestEnvelope = read_frame(&mut stream).await.unwrap();
    atomic_write_json(&path, &replacement).unwrap();
    let response = ResponseEnvelope::success(&request, &test_worker_hello(&authenticated)).unwrap();
    write_frame(&mut stream, &response).await.unwrap();
    drop(stream);
    drop(listener);
    std::fs::remove_file(socket_path).unwrap();
}
