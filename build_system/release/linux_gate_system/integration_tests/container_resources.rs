use super::*;
use yolo_release_filesystem::TemporaryDirectory;

const TOOL_PREFIX: &str = "yolo-mounted-tool-test-";
const TOOL_NAME: &str = "bazel";
const TOOL_BYTES: &[u8] = b"mounted read-only tool";

#[test]
fn external_container_uncertainty_retains_snapshot_and_read_only_mounted_tool() {
    for settled in [false, true] {
        let snapshot = GateSnapshot::fixture().unwrap();
        let root = snapshot.root_path();
        let tool = TemporaryDirectory::create(TOOL_PREFIX).unwrap();
        let tool_root = tool.path();
        let executable = tool_root.join(TOOL_NAME);
        std::fs::write(&executable, TOOL_BYTES).unwrap();
        let engine = TestEngine::default();
        let contract = contract_with_bazel(&snapshot, &executable);
        let owner = Owner::prepare(&snapshot, &engine, contract, Some(tool.handle())).unwrap();
        if settled {
            owner.finish(Ok(())).unwrap();
        } else {
            assert!(owner.finish(Err(anyhow::anyhow!(CLIENT_FAILURE))).is_err());
        }
        drop(tool);
        drop(snapshot);
        assert_eq!(root.exists(), !settled);
        assert_eq!(tool_root.exists(), !settled);
        if !settled {
            assert_eq!(std::fs::read(executable).unwrap(), TOOL_BYTES);
            remove_preserved(root);
            remove_preserved(tool_root);
        }
    }
}
