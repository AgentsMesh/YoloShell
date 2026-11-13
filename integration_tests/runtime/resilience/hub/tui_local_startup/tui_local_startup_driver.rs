use super::{constants::*, fixture::Installation, shell::ShellProbe};
use crate::hub::local_startup_support::Services;
use crate::hub::tui_queries::constants::{CONTROL_READY_HEADER, TERMINAL_PICKER_READY_SCREEN};
use crate::{Harness, runtime_ref_from_view};
#[path = "../federation/federation_tui_capture.rs"]
mod capture;
#[path = "tui_local_startup_command.rs"]
mod command;
#[path = "../federation/federation_tui_lease.rs"]
mod lease;
use crate::hub::federation_support::{RunningNode, deadline};

pub(super) async fn exercise(
    terminal: &Harness,
    host: &Harness,
    installation: &Installation,
    original: &yolo_runtime::model::TerminalView,
) {
    let driver = terminal.create_cat(DRIVER_NAME).await;
    let root_runtime = driver.runtime.as_ref().unwrap();
    let driver_root = crate::ProcessIdentity {
        pid: root_runtime.child_pid,
        start_time: root_runtime.child_start_time,
    };
    assert!(driver_root.matches());
    let shell = ShellProbe::prepare(host, original, installation).await;
    let runtime = runtime_ref_from_view(&driver);
    let client = terminal.client();
    let grant = client
        .acquire_writer(
            runtime,
            DRIVER_NAME,
            Some(yolo_runtime::lease::constants::MAX_TTL_MS),
        )
        .await
        .unwrap();
    let renewal = lease::Renewal::start(client.clone(), runtime, grant.clone());
    let mut output = client
        .attach(
            driver.manifest.terminal_id,
            yolo_runtime::worker::FORCE_SNAPSHOT_AFTER_SEQUENCE,
        )
        .await
        .unwrap();
    let snapshot = output.initial.snapshot.as_ref().unwrap();
    let (rows, cols) = (snapshot.rows, snapshot.cols);
    let mut services: Option<Services> = None;
    let mut connection = None;
    for signal in [None, Some(libc::SIGINT), Some(libc::SIGQUIT)] {
        let mut screen = vt100::Parser::new(rows, cols, 0);
        let launch = command::Launch::new(installation);
        client
            .input(
                runtime,
                grant.generation,
                &grant.writer_capability,
                launch.command.as_bytes().to_vec(),
            )
            .await
            .unwrap();
        capture::wait_screen(&mut output, &mut screen, |text| {
            (text.contains(SHELL_NAME) && text.contains(CONTROL_READY_HEADER))
                || text.contains(&launch.complete)
        })
        .await;
        if screen.screen().contents().contains(&launch.complete) {
            launch.completed();
            panic!("TUI exited before displaying its selected Hub session");
        }
        let process = launch.process(installation, driver_root);
        let hub = yolo_hub_state::HubState::inspect_identity(
            &installation.profile.state,
            &installation.profile.host_home,
        )
        .unwrap()
        .unwrap();
        let socket =
            yolo_hub_state::HubState::socket_path_for(&installation.profile.state).unwrap();
        let current = yolo_hub_local_startup::ConnectionDocument::local(hub, socket);
        if let Some(expected) = &connection {
            assert_eq!(&current, expected);
            let retained = services.as_ref().unwrap();
            let (host, hub) = Services::inspect(&installation.profile.host_home, &current).await;
            assert_eq!(host, retained.host);
            assert_eq!(hub, retained.hub);
        } else {
            services = Some(Services::capture(&installation.profile.host_home, &current).await);
            connection = Some(current);
        }
        if let Some(signal) = signal {
            assert_eq!(launch.process(installation, driver_root), process);
            assert!(process.signal(signal).unwrap());
        } else {
            client
                .input(
                    runtime,
                    grant.generation,
                    &grant.writer_capability,
                    ENTER.to_vec(),
                )
                .await
                .unwrap();
            let title = format!("{SHELL_NAME}{CONTROL_LABEL}");
            capture::wait_screen(&mut output, &mut screen, |text| {
                text.contains(&title) && !text.contains(TERMINAL_PICKER_READY_SCREEN)
            })
            .await;
            client
                .input(
                    runtime,
                    grant.generation,
                    &grant.writer_capability,
                    KEYBOARD_INTERRUPT.to_vec(),
                )
                .await
                .unwrap();
            assert!(process.matches(), "keyboard Ctrl-C must not exit the TUI");
            // Origin receipt must precede view navigation: leaving Terminal
            // deliberately retires any input that has not yet been admitted.
            shell.interrupt_delivered_once().await;
            client
                .input(
                    runtime,
                    grant.generation,
                    &grant.writer_capability,
                    OPEN_FILTER_FROM_TERMINAL.to_vec(),
                )
                .await
                .unwrap();
            capture::wait_screen(&mut output, &mut screen, |text| text.contains(FILTER_LABEL))
                .await;
            client
                .input(
                    runtime,
                    grant.generation,
                    &grant.writer_capability,
                    CLOSE_FILTER.to_vec(),
                )
                .await
                .unwrap();
            capture::wait_screen(&mut output, &mut screen, |text| {
                !text.contains(FILTER_LABEL)
            })
            .await;
            client
                .input(
                    runtime,
                    grant.generation,
                    &grant.writer_capability,
                    QUIT.to_vec(),
                )
                .await
                .unwrap();
        }
        capture::wait_screen(&mut output, &mut screen, |text| {
            text.contains(&launch.complete)
        })
        .await;
        assert!(!process.matches());
        launch.completed();
        shell.after_tui_exit(host).await;
        let retained = services.as_ref().unwrap();
        assert!(retained.host.matches() && retained.hub.matches());
    }
    let grant = renewal.stop().await;
    shell.restore(host).await;
    client.release_writer(&grant, runtime).await.unwrap();
    output.detach().await.unwrap();
    client
        .terminate(driver.manifest.terminal_id, false)
        .await
        .unwrap();
    services.unwrap().stop().await;
}
