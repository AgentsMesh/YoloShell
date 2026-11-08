use super::{constants::*, deadline};
use crate::{Harness, runtime_ref_from_view};
use tokio::time::{sleep, timeout_at};
use yolo_runtime::model::TerminalView;

pub async fn single_shell(host: &Harness) -> TerminalView {
    timeout_at(deadline(), async {
        loop {
            let terminals = host.client().list_sessions_complete(false).await.unwrap();
            assert!(
                terminals.len() <= ONE_SHELL,
                "automatic entry created duplicate shells"
            );
            if let Some(session) = terminals.into_iter().next() {
                return session;
            }
            sleep(POLL).await;
        }
    })
    .await
    .unwrap()
}

pub async fn same_shell(host: &Harness, before: &TerminalView) {
    let after = single_shell(host).await;
    assert_eq!(after.manifest.terminal_id, before.manifest.terminal_id);
    assert_eq!(runtime_ref_from_view(&after), runtime_ref_from_view(before));
    let (before, after) = (
        before.runtime.as_ref().unwrap(),
        after.runtime.as_ref().unwrap(),
    );
    assert_eq!(
        (
            after.worker_pid,
            after.worker_start_time,
            after.child_pid,
            after.child_start_time
        ),
        (
            before.worker_pid,
            before.worker_start_time,
            before.child_pid,
            before.child_start_time
        )
    );
}

pub fn client_ids(output: &str) -> Vec<uuid::Uuid> {
    let mut lines = output
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty());
    assert_eq!(lines.next(), Some(CLIENT_HEADER), "{output}");
    lines
        .map(|line| {
            line.split_whitespace()
                .next()
                .expect("client id column")
                .parse::<uuid::Uuid>()
                .unwrap()
        })
        .collect()
}
