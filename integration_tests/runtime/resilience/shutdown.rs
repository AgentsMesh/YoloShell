use super::*;

const ADOPTION_SESSION_NAME: &str = "sigterm-adoption";
const ADOPTION_TIMEOUT_ERROR: &str = "replacement Host did not adopt the surviving Worker";
const ADOPTION_TIMEOUT: Duration = Duration::from_secs(5);
const ADOPTION_POLL_INTERVAL: Duration = Duration::from_millis(20);

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn sigterm_without_workers_removes_the_host_socket_and_namespace() {
    let home = tempfile::tempdir().unwrap();
    let paths = Paths::new(home.path());
    let host_socket = paths.host_socket().unwrap();
    let namespace = host_socket.parent().unwrap().to_path_buf();
    let mut host = HostGuard::new(spawn_host(home.path()));
    wait_for_host(home.path(), &mut host).await;
    assert!(host_socket.exists());

    host.stop_with_sigterm();

    assert!(!host_socket.exists());
    assert!(!namespace.exists());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn sigterm_preserves_a_worker_until_a_new_host_adopts_it() {
    let namespace;
    {
        let mut harness = Harness::start().await;
        let created = harness.create_cat(ADOPTION_SESSION_NAME).await;
        let terminal_id = created.manifest.terminal_id;
        let paths = harness.paths();
        let before: RuntimeRecord = read_json(&paths.runtime_record(terminal_id)).unwrap();
        let worker = ProcessIdentity {
            pid: before.worker_pid,
            start_time: before.worker_start_time,
        };
        let child = ProcessIdentity {
            pid: before.child_pid,
            start_time: before.child_start_time,
        };
        let host_socket = paths.host_socket().unwrap();
        namespace = host_socket.parent().unwrap().to_path_buf();
        let namespace_authority = paths.retain_socket_namespace().unwrap();
        let namespace_directory = namespace_authority.directory().unwrap();
        let before_directory = namespace_directory.object_identity().unwrap();
        namespace_authority.verify().unwrap();
        drop(namespace_authority);

        harness.stop_host_with_sigterm();

        assert!(!host_socket.exists());
        assert!(worker.matches());
        assert!(child.matches());
        assert!(before.socket_path.exists());
        namespace_directory.verify_path().unwrap();
        assert_eq!(paths.socket_dir().unwrap(), namespace);
        assert!(namespace.is_dir());

        harness.replace_host(spawn_host(harness.home.path())).await;
        let after = wait_for_adoption(&harness, terminal_id, before.controller_generation).await;
        assert_eq!(after.worker_pid, before.worker_pid);
        assert_eq!(after.worker_start_time, before.worker_start_time);
        assert_eq!(after.child_pid, before.child_pid);
        assert_eq!(after.child_start_time, before.child_start_time);
        assert_eq!(after.socket_path, before.socket_path);
        assert!(worker.matches());
        assert!(child.matches());
        assert!(host_socket.exists());
        namespace_directory.verify_path().unwrap();
        assert_eq!(paths.socket_dir().unwrap(), namespace);
        assert_eq!(
            namespace_directory.object_identity().unwrap(),
            before_directory
        );
        #[cfg(target_os = "macos")]
        let guardian = adopted_guardian(&paths, terminal_id);

        harness.client().terminate(terminal_id, false).await.unwrap();
        wait_for_identity_gone(worker).await;
        wait_for_identity_gone(child).await;
        #[cfg(target_os = "macos")]
        wait_for_identity_gone(guardian).await;
    }
    assert!(
        !namespace.exists(),
        "runtime socket namespace survived harness cleanup: entries={:?}",
        std::fs::read_dir(&namespace).map(|entries| entries
            .filter_map(|entry| entry.ok().map(|entry| entry.file_name()))
            .collect::<Vec<_>>())
    );
}

#[cfg(target_os = "macos")]
fn adopted_guardian(paths: &Paths, terminal_id: Uuid) -> ProcessIdentity {
    let guardian = yolo_resilience_support::secondary_custody_adversarial_observation::
        ActiveCustodyObservation::read(paths, terminal_id)
        .unwrap()
        .guardian();
    ProcessIdentity {
        pid: guardian.pid,
        start_time: guardian.start_time,
    }
}

async fn wait_for_adoption(
    harness: &Harness,
    terminal_id: Uuid,
    previous_generation: u64,
) -> RuntimeRecord {
    let deadline = tokio::time::Instant::now() + ADOPTION_TIMEOUT;
    loop {
        harness.client().reconcile().await.unwrap();
        let record: RuntimeRecord = read_json(&harness.paths().runtime_record(terminal_id)).unwrap();
        if record.controller_generation > previous_generation {
            return record;
        }
        assert!(
            tokio::time::Instant::now() < deadline,
            "{}",
            ADOPTION_TIMEOUT_ERROR
        );
        tokio::time::sleep(ADOPTION_POLL_INTERVAL).await;
    }
}
