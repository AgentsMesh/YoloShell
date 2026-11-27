use super::{federation_support::*, management_support::ManagedNode};
use std::{
    path::{Path, PathBuf},
    process::Stdio,
};
use tokio::{io::AsyncReadExt, time::timeout_at};
use yolo_hub_federation_settings::Peer;
use yolo_hub_identity::TerminalAddress;

const PROBE_ENV: &str = "YOLO_TEST_NATIVE_HUB_MANAGEMENT_BINARY";
const ATTACH_ENV: &str = "YOLO_TEST_HUB_ATTACH_BINARY";
const OUTPUT_ENV: &str = "TEST_UNDECLARED_OUTPUTS_DIR";
const INPUT_FILE: &str = "native-management.json";
pub const REPORT_FILE: &str = "native-management-report.json";
const IMAGE_FILE: &str = "hub-management.png";
const FILE_DIRECTORY: &str = "workspace ";
const CHANGED_FILE_DIRECTORY: &str = "workspace-next ";
const MAX_DIAGNOSTIC_BYTES: u64 = 64 * 1024;
const PNG_SIGNATURE: &[u8] = b"\x89PNG\r\n\x1a\n";

#[derive(serde::Serialize)]
struct Input {
    connection: yolo_hub_local_startup::ConnectionDocument,
    peer: Peer,
    session: TerminalAddress,
    file_directory: String,
    changed_file_directory: String,
    protected_directory: String,
}

pub async fn run(node: &ManagedNode, root: &Path, peer: Peer, session: TerminalAddress) {
    let input = root.join(INPUT_FILE);
    let report = root.join(REPORT_FILE);
    let image = std::env::var_os(OUTPUT_ENV)
        .map(PathBuf::from)
        .unwrap_or_else(|| root.to_path_buf())
        .join(IMAGE_FILE);
    let connection =
        yolo_hub_local_startup::ConnectionDocument::local(node.hub, node.socket.clone());
    let file_directory = root.join(FILE_DIRECTORY);
    let changed_file_directory = root.join(CHANGED_FILE_DIRECTORY);
    std::fs::create_dir(&file_directory).unwrap();
    std::fs::create_dir(&changed_file_directory).unwrap();
    yolo_runtime::paths::atomic_write_json(
        &input,
        &Input {
            connection,
            peer,
            session,
            file_directory: file_directory
                .canonicalize()
                .unwrap()
                .to_str()
                .unwrap()
                .into(),
            changed_file_directory: changed_file_directory
                .canonicalize()
                .unwrap()
                .to_str()
                .unwrap()
                .into(),
            protected_directory: root.canonicalize().unwrap().to_str().unwrap().into(),
        },
    )
    .unwrap();
    let binary = |name| crate::bazel_runfile(&std::env::var(name).unwrap()).unwrap();
    let mut child = tokio::process::Command::new(binary(PROBE_ENV))
        .arg(input)
        .arg(binary(ATTACH_ENV))
        .arg(report)
        .arg(&image)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .kill_on_drop(true)
        .spawn()
        .unwrap();
    let mut errors = child.stderr.take().unwrap().take(MAX_DIAGNOSTIC_BYTES);
    let mut diagnostic = Vec::new();
    let result = timeout_at(deadline(), async {
        tokio::try_join!(child.wait(), errors.read_to_end(&mut diagnostic))
    })
    .await;
    match result {
        Ok(Ok((status, _))) => assert!(
            status.success(),
            "native management failed: {}",
            String::from_utf8_lossy(&diagnostic)
        ),
        result => {
            let _ = child.kill().await;
            let _ = child.wait().await;
            panic!(
                "native management did not finish: {result:?}; {}",
                String::from_utf8_lossy(&diagnostic)
            );
        }
    }
    let bytes = std::fs::read(image).unwrap();
    assert!(bytes.starts_with(PNG_SIGNATURE));
}
