pub const SAFE_BRIDGE: &str = r#"
impl Drop for SpawnedHost {
    fn drop(&mut self) {
        let Some(mut child) = self.child.take() else { return; };
        let observed = ::std::process::Child::try_wait(&mut child);
        match observed {
            Ok(Some(_)) => {}
            Ok(None) => {
                let terminated = ::std::process::Child::kill(&mut child);
                if let Err(error) = terminated { settle_failed_child(child, error); return; }
                if let Err(error) = ::std::process::Child::wait(&mut child) { handoff(child, error); }
            }
            Err(error) => settle_failed_child(child, error),
        }
    }
}
"#;

pub const SPAWNED_PATH: &str = "core/crates/application/broker/child/custody/src/pending.rs";
pub const SETTLEMENT_PATH: &str = "core/crates/application/broker/child/custody/src/spawned_settlement.rs";
pub const REAPER_CUSTODY_PATH: &str =
    "core/crates/application/broker/child/custody/src/spawned_termination_custody.rs";
pub const REAPER_WORKER_PATH: &str =
    "core/crates/application/broker/child/custody/src/spawned_termination_worker.rs";
const REMOTE_CONTRACT_PATHS: &[&str] = &[
    SPAWNED_PATH,
    "core/crates/application/broker/child/custody/src/pending_lifecycle.rs",
    "core/crates/application/broker/child/custody/src/pending_supervision.rs",
    "core/crates/application/broker/child/custody/src/spawned_failure.rs",
    "core/crates/application/broker/child/custody/src/spawned_result.rs",
    SETTLEMENT_PATH,
    "core/crates/application/broker/child/custody/src/spawned_state.rs",
    "core/crates/application/broker/child/custody/src/supervised_broker.rs",
    "core/crates/application/broker/child/custody/src/supervised_containment.rs",
    "core/crates/application/broker/child/custody/src/supervised_wait.rs",
    "core/crates/application/broker/child/custody/src/spawned_termination_reaper.rs",
    "core/crates/application/broker/child/custody/src/spawned_termination_containment.rs",
    REAPER_CUSTODY_PATH,
    REAPER_WORKER_PATH,
];
const CONFORMANCE_ROOT_ENV: &str = "YOLO_ARCH_CONFORMANCE_ROOT";
const TEST_SOURCE_DIRECTORY_ENV: &str = "TEST_SRCDIR";
const TEST_WORKSPACE_ENV: &str = "TEST_WORKSPACE";

pub fn remote_source(path: &str) -> String {
    let root = std::env::var_os(CONFORMANCE_ROOT_ENV)
        .map(std::path::PathBuf::from)
        .or_else(|| {
            let source = std::env::var_os(TEST_SOURCE_DIRECTORY_ENV)?;
            let workspace = std::env::var_os(TEST_WORKSPACE_ENV)?;
            Some(std::path::PathBuf::from(source).join(workspace))
        })
        .expect("process-signal contract tests require Bazel runfiles");
    std::fs::read_to_string(root.join(path)).unwrap()
}

pub fn write_remote_contract(fixture: &super::fixture::Fixture) {
    for path in REMOTE_CONTRACT_PATHS {
        fixture.write(path, remote_source(path));
    }
}
