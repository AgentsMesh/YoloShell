const UNCERTAIN_EXEC_HOST_ROLE_ENV: &str = "TEST_UNCERTAIN_EXEC_HOST_ROLE";
const UNCERTAIN_EXEC_HOST_ROLE: &str = "holder";
const UNCERTAIN_EXEC_HOST_HOME_ENV: &str = "TEST_UNCERTAIN_EXEC_HOST_HOME";
const UNCERTAIN_EXEC_HOST_SESSION_ENV: &str = "TEST_UNCERTAIN_EXEC_HOST_SESSION";
const UNCERTAIN_EXEC_HOST_READY_ENV: &str = "TEST_UNCERTAIN_EXEC_HOST_READY";
const UNCERTAIN_EXEC_HOST_OUTCOME_ENV: &str = "TEST_UNCERTAIN_EXEC_HOST_OUTCOME";
const UNCERTAIN_EXEC_HOST_EXIT_ENV: &str = "TEST_UNCERTAIN_EXEC_HOST_EXIT";
#[cfg(feature = "yolo-unit-tests")]
const UNCERTAIN_EXEC_HOST_TEST: &str =
    "worker::tests::uncertain_exec_custody_holder_process_unit";
#[cfg(feature = "yolo-integration-tests")]
const UNCERTAIN_EXEC_HOST_TEST: &str =
    "worker::tests::uncertain_exec_custody_holder_process_integration";
const UNCERTAIN_EXEC_HOST_READY: &[u8] = b"ready";
const UNCERTAIN_EXEC_HOST_NO_DECISION: &[u8] = b"no-decision";
const UNCERTAIN_EXEC_HOST_RELEASE: &[u8] = b"release";
const UNCERTAIN_EXEC_HOST_ACTIVATED: &[u8] = b"activated";
const UNCERTAIN_EXEC_HOST_TRANSPORT_ERROR: &[u8] = b"transport-error";
const UNCERTAIN_EXEC_HOST_WAIT_DETAIL: &str = "uncertain-exec Host helper did not converge";
const UNCERTAIN_EXEC_HOST_WAIT_TIMEOUT: Duration = crate::session_timing::WORKER_READY_TIMEOUT;
const UNCERTAIN_EXEC_HOST_READY_FILE: &str = "uncertain-exec-host.ready";
const UNCERTAIN_EXEC_HOST_OUTCOME_FILE: &str = "uncertain-exec-host.outcome";
const UNCERTAIN_EXEC_HOST_EXIT_FILE: &str = "uncertain-exec-host.exit";
include!("unit/runtime/runtime_exec_report_custody_connect.rs");

struct UncertainExecHost {
    child: Option<std::process::Child>,
    ready: std::path::PathBuf,
    outcome: std::path::PathBuf,
    exit: std::path::PathBuf,
}

impl UncertainExecHost {
    fn spawn(paths: &Paths, terminal_id: Uuid) -> Self {
        let directory = paths.session_dir(terminal_id);
        let ready = directory.join(UNCERTAIN_EXEC_HOST_READY_FILE);
        let outcome = directory.join(UNCERTAIN_EXEC_HOST_OUTCOME_FILE);
        let exit = directory.join(UNCERTAIN_EXEC_HOST_EXIT_FILE);
        let child = crate::process::spawn_waitable_test_child(
            std::process::Command::new(std::env::current_exe().unwrap())
                .args([
                    crate::constants::TEST_HARNESS_EXACT_ARGUMENT,
                    UNCERTAIN_EXEC_HOST_TEST,
                    crate::constants::TEST_HARNESS_NOCAPTURE_ARGUMENT,
                    crate::constants::TEST_HARNESS_SINGLE_THREAD_ARGUMENT,
                ])
                .env(UNCERTAIN_EXEC_HOST_ROLE_ENV, UNCERTAIN_EXEC_HOST_ROLE)
                .env(UNCERTAIN_EXEC_HOST_HOME_ENV, paths.home())
                .env(UNCERTAIN_EXEC_HOST_SESSION_ENV, terminal_id.to_string())
                .env(UNCERTAIN_EXEC_HOST_READY_ENV, &ready)
                .env(UNCERTAIN_EXEC_HOST_OUTCOME_ENV, &outcome)
                .env(UNCERTAIN_EXEC_HOST_EXIT_ENV, &exit),
        );
        Self {
            child: Some(child),
            ready,
            outcome,
            exit,
        }
    }

    async fn wait_for_no_decision(&self) {
        wait_for_uncertain_exec_marker(&self.ready, UNCERTAIN_EXEC_HOST_READY).await;
        wait_for_uncertain_exec_marker(&self.outcome, UNCERTAIN_EXEC_HOST_NO_DECISION).await;
    }

    async fn release(mut self) {
        publish_uncertain_exec_marker(&self.exit, UNCERTAIN_EXEC_HOST_READY);
        let status = tokio::time::timeout(RUNTIME_ERROR_CUSTODY_TIMEOUT, async {
            loop {
                if let Some(status) = self.child.as_mut().unwrap().try_wait().unwrap() {
                    return status;
                }
                tokio::time::sleep(RUNTIME_ERROR_CUSTODY_POLL_INTERVAL).await;
            }
        })
        .await
        .expect(UNCERTAIN_EXEC_HOST_WAIT_DETAIL);
        self.child = None;
        assert!(status.success());
    }
}

#[cfg(test)]
impl Drop for UncertainExecHost {
    #[cfg(test)]
    fn drop(&mut self) {
        if let Some(child) = self.child.as_mut() {
            let _ = child.kill();
            if let Ok(status) = child.wait() {
                yolo_coverage_test_support::discard_reaped_kill_profile(child.id(), status).unwrap();
            }
        }
    }
}

fn run_uncertain_exec_custody_holder_process() {
    if std::env::var(UNCERTAIN_EXEC_HOST_ROLE_ENV).as_deref() != Ok(UNCERTAIN_EXEC_HOST_ROLE) {
        return;
    }
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(run_uncertain_exec_host())
        .unwrap();
}

#[cfg(all(feature = "yolo-unit-tests", feature = "yolo-worker-unit-runtime"))]
include!("../unit_tests/uncertain_exec_host_entrypoint.rs");
#[cfg(feature = "yolo-integration-tests")]
include!("../integration_tests/uncertain_exec_host_entrypoint.rs");

async fn run_uncertain_exec_host() -> Result<()> {
    let paths = Paths::new(std::env::var_os(UNCERTAIN_EXEC_HOST_HOME_ENV).unwrap());
    let terminal_id = std::env::var(UNCERTAIN_EXEC_HOST_SESSION_ENV)
        .unwrap()
        .parse()
        .unwrap();
    let ready = std::env::var_os(UNCERTAIN_EXEC_HOST_READY_ENV).unwrap();
    let outcome = std::env::var_os(UNCERTAIN_EXEC_HOST_OUTCOME_ENV).unwrap();
    let exit = std::env::var_os(UNCERTAIN_EXEC_HOST_EXIT_ENV).unwrap();
    let record = wait_for_runtime_error_record(&paths, terminal_id)
        .await
        .unwrap();
    let holder = crate::pty_custody::CustodyHolderIdentity::capture().unwrap();
    let request = crate::pty_custody::CustodyRequest::new(
        crate::pty_custody::CustodyLeaseId::new(),
        crate::pty_custody::CustodyRuntimeIdentity::from_record(&record),
        holder,
        record.auth_token.clone(),
        record.controller_generation,
    );
    let (mut stream, offer) = connect_uncertain_exec_host(&record, &request).await;
    let descriptors = stream
        .receive_rights(offer.descriptor_set.descriptor_count())
        .await
        .unwrap();
    let _custody = import_runtime_error_custody(&record, &offer, descriptors)?;
    let prepared = initial_test_prepared_ack(&paths, &request, &offer, &_custody).await?;
    stream.write(&prepared).await.unwrap();
    let commit: crate::pty_custody::CustodyWorkerCommit = stream.read().await.unwrap();
    assert!(commit.matches_offer(&offer));
    stream
        .write(&crate::pty_custody::CustodyHostAck::ready(
            &offer,
            request.controller_generation,
        ))
        .await
        .unwrap();
    publish_uncertain_exec_marker(std::path::Path::new(&ready), UNCERTAIN_EXEC_HOST_READY);
    let disposition = stream
        .read::<crate::pty_custody::CustodyWorkerReadyDisposition>()
        .await;
    let marker = match disposition {
        Err(error)
            if matches!(
                error.downcast_ref::<crate::protocol::ProtocolError>(),
                Some(crate::protocol::ProtocolError::EndOfStream)
            ) =>
        {
            UNCERTAIN_EXEC_HOST_NO_DECISION
        }
        Err(_) => UNCERTAIN_EXEC_HOST_TRANSPORT_ERROR,
        Ok(crate::pty_custody::CustodyWorkerReadyDisposition::Release { .. }) => {
            UNCERTAIN_EXEC_HOST_RELEASE
        }
        Ok(crate::pty_custody::CustodyWorkerReadyDisposition::Activated { .. }) => {
            UNCERTAIN_EXEC_HOST_ACTIVATED
        }
    };
    publish_uncertain_exec_marker(std::path::Path::new(&outcome), marker);
    wait_for_uncertain_exec_marker(exit.as_ref(), UNCERTAIN_EXEC_HOST_READY).await;
    Ok(())
}

fn publish_uncertain_exec_marker(path: &std::path::Path, marker: &[u8]) {
    atomic_write_json(path, &marker).unwrap();
}

async fn wait_for_uncertain_exec_marker(path: &std::path::Path, expected: &[u8]) {
    let actual = tokio::time::timeout(UNCERTAIN_EXEC_HOST_WAIT_TIMEOUT, async {
        loop {
            if let Ok(actual) = read_json::<Vec<u8>>(path) {
                return actual;
            }
            tokio::time::sleep(RUNTIME_ERROR_CUSTODY_POLL_INTERVAL).await;
        }
    })
    .await
    .expect(UNCERTAIN_EXEC_HOST_WAIT_DETAIL);
    assert_eq!(actual, expected);
}
