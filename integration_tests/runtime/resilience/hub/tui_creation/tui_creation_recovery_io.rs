use super::constants::*;
use crate::hub::support::{MAX_OBSERVED_BYTES, deadline};
use crate::{Harness, runtime_ref_from_view};
use yolo_runtime::{
    model::TerminalView,
    process::ProcessIdentity,
    protocol::{event, from_value},
};

pub async fn alive_with_fresh_output(host: &Harness, before: &TerminalView) {
    let runtime = before.runtime.as_ref().unwrap();
    for (pid, start_time) in [
        (runtime.worker_pid, runtime.worker_start_time),
        (runtime.child_pid, runtime.child_start_time),
    ] {
        assert!(ProcessIdentity { pid, start_time }.matches());
    }
    let target = runtime_ref_from_view(before);
    let client = host.client();
    let grant = client
        .acquire_writer(
            target,
            PROBE_WRITER,
            Some(yolo_runtime::lease::constants::MAX_TTL_MS),
        )
        .await
        .unwrap();
    let mut output = client
        .attach(
            target.terminal_id,
            yolo_runtime::worker::FORCE_SNAPSHOT_AFTER_SEQUENCE,
        )
        .await
        .unwrap();
    let nonce = uuid::Uuid::new_v4();
    let marker = format!("{MARKER_PREFIX}{nonce}");
    let command = format!("printf '%s%s\\n' '{MARKER_PREFIX}' '{nonce}'\r");
    client
        .input(
            target,
            grant.generation,
            &grant.writer_capability,
            command.into_bytes(),
        )
        .await
        .unwrap();
    let mut bytes = Vec::new();
    tokio::time::timeout_at(deadline(), async {
        loop {
            let event = output.next_event().await.unwrap();
            assert_ne!(event.event, event::SESSION_EXITED);
            assert_ne!(event.event, event::OUTPUT_GAP);
            if event.event == event::TERMINAL_OUTPUT {
                let chunk: yolo_runtime::model::OutputChunk = from_value(event.payload).unwrap();
                bytes.extend_from_slice(&chunk.data);
                assert!(bytes.len() <= MAX_OBSERVED_BYTES);
                if String::from_utf8_lossy(&bytes).contains(&marker) {
                    break;
                }
            }
        }
    })
    .await
    .unwrap();
    client.release_writer(&grant, target).await.unwrap();
    output.detach().await.unwrap();
}
