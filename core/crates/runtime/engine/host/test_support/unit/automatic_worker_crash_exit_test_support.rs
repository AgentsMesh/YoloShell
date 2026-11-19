#[cfg(target_os = "macos")]
const AUTOMATIC_EXIT_TEST_RUNTIME_SOCKET: &str = "automatic-exit-runtime.sock";
#[cfg(target_os = "macos")]
const AUTOMATIC_EXIT_TEST_OUTPUT: &[u8] = b"automatic-exit-output";
#[cfg(target_os = "macos")]
const AUTOMATIC_EXIT_TEST_PREVIOUS_BOOT: &str = "automatic-exit-previous-boot";
#[cfg(target_os = "macos")]
const AUTOMATIC_EXIT_TEST_EMPTY_SEQUENCE: u64 = 0;

#[cfg(target_os = "macos")]
fn overwrite_automatic_exit_test_json<T: Serialize>(path: &Path, value: &T) -> Result<()> {
    let encoded = serde_json::to_vec(value)?;
    write_private_bytes(path, &encoded);
    Ok(())
}

#[cfg(target_os = "macos")]
struct AutomaticCrashFixture {
    host: Host,
    manifest: TerminalManifest,
    runtime: RuntimeRecord,
    _directory: HostTestDirectory,
}

#[cfg(target_os = "macos")]
impl AutomaticCrashFixture {
    fn new() -> Self {
        let directory = tempdir().unwrap();
        let host = Host::open(HostConfig::new(directory.path())).unwrap();
        let manifest = fixture_manifest(directory.path());
        let mut runtime = host_runtime_record(
            &host,
            manifest.terminal_id,
            directory.path().join(AUTOMATIC_EXIT_TEST_RUNTIME_SOCKET),
        );
        let worker = reaped_test_process();
        let child = reaped_test_process();
        runtime.worker_pid = worker.pid;
        runtime.worker_start_time = worker.start_time;
        runtime.child_pid = child.pid;
        runtime.child_start_time = child.start_time;
        persist_host_session(&host, &manifest, &runtime);
        Self {
            _directory: HostTestDirectory::capture(directory, &host.paths),
            host,
            manifest,
            runtime,
        }
    }

    fn settle_custody(&self) {
        self.host.secondary_custody.install_test_retired_lease(
            &self.host,
            &self.runtime,
            secondary_custody::TestRetiredCustodyState::Released,
        );
    }

    fn append_output(&self) -> u64 {
        let mut journal = crate::journal::Journal::open(
            self.host.paths.journal(self.manifest.terminal_id),
            self.runtime.runtime_epoch,
        )
        .unwrap();
        journal.append(AUTOMATIC_EXIT_TEST_OUTPUT).unwrap().sequence
    }

    fn remove_journal_evidence(&self) {
        let journal_path = self.host.paths.journal(self.manifest.terminal_id);
        let cursor_path = crate::journal::retained_cursor_path_for_test(&journal_path);
        std::fs::remove_file(cursor_path).unwrap();
        std::fs::remove_file(journal_path).unwrap();
    }

    fn write_exit(&self, reason: ExitReason, runtime_epoch: Uuid, last_output_sequence: u64) {
        let exit = ExitRecord {
            runtime_epoch,
            code: None,
            signal: None,
            reason,
            observed_at_ms: now_ms(),
            last_output_sequence,
        };
        atomic_write_json(
            &self.host.paths.exit_record(self.manifest.terminal_id),
            &exit,
        )
        .unwrap();
    }
}
