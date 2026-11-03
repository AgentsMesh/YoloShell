use super::support::{RunningHub, SOCKET_TEST_ROOT, STATE_DIRECTORY, deadline};
use crate::Harness;
#[path = "tui_shell_first/constants.rs"]
mod constants;
#[path = "tui_shell_first_driver.rs"]
pub(super) mod driver;
#[path = "tui_shell_first_lifecycle.rs"]
mod lifecycle;
#[path = "tui_shell_first_observe.rs"]
mod observe;
#[path = "tui_shell_first/presentation.rs"]
mod presentation;
use constants::*;
use driver::Driver;

#[tokio::test(flavor = "multi_thread")]
async fn real_cli_shell_first_detach_attach_and_parallel_clients_preserve_one_pty() {
    let host = Harness::start().await;
    assert!(
        host.client()
            .list_sessions_complete(false)
            .await
            .unwrap()
            .is_empty()
    );
    let root = tempfile::tempdir_in(SOCKET_TEST_ROOT).unwrap();
    let directory = root.path().canonicalize().unwrap();
    let hub = RunningHub::start(&host, &directory.join(STATE_DIRECTORY)).await;
    let state = directory.join(CLIENT_DIRECTORY);
    let driver_host = Harness::start().await;
    let mut first = Driver::new(&driver_host).await;
    let mut second = Driver::new(&driver_host).await;
    let mut control = Driver::new(&driver_host).await;

    first.launch(&hub, &state, AUTO_COMMAND).await;
    first.wait(terminal_ready).await;
    let original = observe::single_shell(&host).await;
    host.track_view(&original);
    presentation::title_and_notification(&mut first, &host, &original).await;
    first.send(SHELL_MARKER_COMMAND).await;
    first.wait(|text| text.contains(SHELL_MARKER)).await;
    // q starts as a standalone ordinary key; the following shell function must receive it.
    first.send(DEFINE_Q_COMMAND).await;
    first.wait(|text| text.contains(Q_READY_MARKER)).await;
    first.send(ORDINARY_Q).await;
    first.send(ENTER).await;
    first.wait(|text| text.contains(Q_MARKER)).await;
    first.send(DETACH_KEYS).await;
    first.returned().await;
    observe::same_shell(&host, &original).await;

    first.launch(&hub, &state, ATTACH_COMMAND).await;
    first.wait(terminal_ready).await;
    first.send(RETURN_MARKER_COMMAND).await;
    first.wait(|text| text.contains(RETURN_MARKER)).await;
    observe::same_shell(&host, &original).await;
    let first_id = observe::client_ids(&control.execute(&hub, &state, CLIENTS_COMMAND).await);
    assert_eq!(first_id.len(), ONE_CLIENT);

    second.launch(&hub, &state, ATTACH_COMMAND).await;
    second
        .wait(|text| {
            text.lines()
                .last()
                .is_some_and(|line| line.contains(INPUT_HELD) && line.contains(TAKE_CONTROL_HINT))
        })
        .await;
    let ids = observe::client_ids(&control.execute(&hub, &state, CLIENTS_COMMAND).await);
    assert_eq!(ids.len(), TWO_CLIENTS);
    assert!(ids.contains(&first_id[0]));
    let second_id = ids.into_iter().find(|id| *id != first_id[0]).unwrap();
    let detached = control
        .execute(&hub, &state, &format!("{DETACH_COMMAND} {second_id}"))
        .await;
    assert!(detached.contains(DETACHED_LABEL));
    second.returned().await;
    observe::same_shell(&host, &original).await;

    let detached = control.execute(&hub, &state, DETACH_COMMAND).await;
    assert!(detached.contains(DETACHED_LABEL));
    first.returned().await;
    // The external command's success ACK permits immediate reattachment, without polling a lock.
    second.launch(&hub, &state, ATTACH_COMMAND).await;
    second.wait(terminal_ready).await;
    second.send(FINAL_MARKER_COMMAND).await;
    second.wait(|text| text.contains(FINAL_MARKER)).await;
    observe::same_shell(&host, &original).await;
    second.send(DETACH_KEYS).await;
    second.returned().await;
    let remaining = observe::client_ids(&control.execute(&hub, &state, CLIENTS_COMMAND).await);
    assert!(remaining.is_empty());

    second.launch(&hub, &state, ATTACH_COMMAND).await;
    second.wait(terminal_ready).await;
    second.send(EXIT_COMMAND).await;
    second
        .wait(|text| text.contains(EXIT_MARKER) && text.contains(ENDED_LABEL))
        .await;
    lifecycle::completed_terminal_moves_to_history(&mut second, &host, &original).await;
    second.send(DETACH_KEYS).await;
    second.returned().await;
    // A fresh client can inspect a completed PTY without acquiring input or
    // silently creating another terminal. Its saved output is still visible.
    first.launch(&hub, &state, ATTACH_COMMAND).await;
    first
        .wait(|text| text.contains(EXIT_MARKER) && text.contains(ENDED_LABEL))
        .await;
    let ended = observe::single_shell(&host).await;
    assert_eq!(ended.manifest.terminal_id, original.manifest.terminal_id);
    first.send(DETACH_KEYS).await;
    first.returned().await;

    // Explicit historical addressing still opens retained output, independently
    // of the active picker and without manufacturing another live terminal.
    let exact = format!("{ATTACH_COMMAND} {}", original.manifest.terminal_id);
    second.launch(&hub, &state, &exact).await;
    second
        .wait(|text| text.contains(EXIT_MARKER) && text.contains(ENDED_LABEL))
        .await;
    let retained = observe::single_shell(&host).await;
    lifecycle::assert_same_completion(&retained, &ended);
    second.send(DETACH_KEYS).await;
    second.returned().await;

    first.close().await;
    second.close().await;
    control.close().await;
    hub.stop().await;
}

fn terminal_ready(text: &str) -> bool {
    // The picker has its own connection glyph before a shell exists.
    // Only the final status row proves a writable terminal at its current size.
    text.lines()
        .last()
        .is_some_and(|line| line.contains(CONTROL))
}
