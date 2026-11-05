use super::*;

const UNSUPPORTED_PROVIDER: &str = "unsupported reboot fixture provider";

pub(super) fn persist_previous_boot_runtime(paths: &Paths, record: &RuntimeRecord) {
    let path = paths.agent_binding(record.terminal_id);
    let binding: yolo_runtime::agent::AgentBinding = read_json(&path).unwrap();
    let rewrite = match binding.kind {
        yolo_runtime::agent::AgentKind::ClaudeCode => {
            yolo_runtime::agent::claude::generation_test_hook::persist_native_previous_boot
        }
        yolo_runtime::agent::AgentKind::Codex => {
            yolo_runtime::agent::codex::authority_test_hook::persist_native_previous_boot
        }
        yolo_runtime::agent::AgentKind::Other(_) => panic!("{UNSUPPORTED_PROVIDER}"),
    };
    rewrite(
        &path,
        record.runtime_epoch,
        &record.boot_id,
        PREVIOUS_BOOT_ID,
    )
    .unwrap();
    yolo_runtime::durable_reboot::adversarial::persist_previous_boot(
        paths,
        record,
        PREVIOUS_BOOT_ID,
    )
    .unwrap();
}

pub(super) async fn persist_binding_state(paths: &Paths, fixture: &super::matrix::RecoveryFixture) {
    let terminal_id = fixture.previous.manifest.terminal_id;
    let path = paths.agent_binding(terminal_id);
    let mut binding: yolo_runtime::agent::AgentBinding = read_json(&path).unwrap();
    binding.state = fixture.state;
    if fixture.state == AgentState::Exited {
        binding.process_pid = None;
        binding.process_start_time = None;
    }
    let deadline = tokio::time::Instant::now() + BINDING_WRITE_TIMEOUT;
    loop {
        let write = match binding.kind {
            yolo_runtime::agent::AgentKind::ClaudeCode => {
                use yolo_runtime::agent::claude::generation_test_hook;
                if fixture.revoked() {
                    generation_test_hook::persist_native_revoked(&path)
                } else {
                    generation_test_hook::persist_native_durable_state(&path, fixture.state)
                }
            }
            yolo_runtime::agent::AgentKind::Codex => {
                use yolo_runtime::agent::codex::authority_test_hook;
                if fixture.revoked() {
                    authority_test_hook::persist_native_revoked(&path)
                } else {
                    authority_test_hook::persist_native_durable_state(&path, fixture.state)
                }
            }
            yolo_runtime::agent::AgentKind::Other(_) => {
                yolo_runtime::paths::atomic_write_json(&path, &binding)
            }
        };
        match write {
            Ok(()) => return,
            Err(error) => {
                assert!(
                    tokio::time::Instant::now() < deadline,
                    "durable binding state write did not settle: {error:#}"
                );
                tokio::time::sleep(POLL_INTERVAL).await;
            }
        }
    }
}
