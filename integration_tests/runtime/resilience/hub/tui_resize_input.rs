use super::{
    support::{RunningHub, SOCKET_TEST_ROOT, STATE_DIRECTORY},
    tui_queries::{
        constants::{EMPTY_OUTPUT, FIRST_NAME},
        driver::Driver,
        source::Source,
    },
    tui_workspaces::support::lease,
};
use crate::Harness;

const CONTROL: &str = " | Control: this UI";
const INTERRUPTED: &str = "Input paused";
const MARKER: &[u8] = b"YOLO_RESIZE_INPUT_OK";
const SIZES: &[(u16, u16)] = &[(144, 36), (176, 44), (152, 39), (168, 42)];
const INPUT: &[u8] = b"resize-input;";

#[tokio::test(flavor = "multi_thread")]
async fn real_tui_resize_during_input_preserves_writer_and_bytes() {
    let host = Harness::start().await;
    let root = tempfile::tempdir_in(SOCKET_TEST_ROOT).unwrap();
    let hub = RunningHub::start(
        &host,
        &root.path().canonicalize().unwrap().join(STATE_DIRECTORY),
    )
    .await;
    let mut source = Source::start(&host, hub.hub, FIRST_NAME).await;
    let driver_host = Harness::start().await;
    let mut driver = Driver::start(&driver_host, &hub, tempfile::tempdir().unwrap()).await;
    driver.open(source.name, CONTROL).await;
    let before = lease(&hub, source.target).await.unwrap();
    let mut expected = Vec::new();
    for &(cols, rows) in SIZES {
        driver.resize_with_input(cols, rows, INPUT).await;
        expected.extend_from_slice(INPUT);
    }
    source.expect(EMPTY_OUTPUT, &expected).await;
    driver.access(source.name, CONTROL).await;
    source.expect(MARKER, &expected).await;
    let title = format!("{}{CONTROL}", source.name);
    driver
        .wait(|text| {
            text.contains(std::str::from_utf8(MARKER).unwrap())
                && text.contains(&title)
                && !text.contains(INTERRUPTED)
        })
        .await;
    let after = lease(&hub, source.target).await.unwrap();
    assert_eq!(before.fields().generation, after.fields().generation);
    assert_eq!(before.fields().holder_id, after.fields().holder_id);
    assert_eq!(
        before.fields().controller_instance_id,
        after.fields().controller_instance_id
    );
    driver.finish().await;
    hub.stop().await;
}
