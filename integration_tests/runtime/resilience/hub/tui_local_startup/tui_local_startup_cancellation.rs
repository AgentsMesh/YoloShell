use super::{constants::*, fixture::Installation};
use crate::hub::local_startup_support::Services;
#[path = "tui_local_startup_cancellation_process.rs"]
mod process;

#[tokio::test(flavor = "multi_thread")]
async fn cli_interrupt_and_quit_cancel_only_the_pending_owned_bootstrap() {
    let survivor = crate::Harness::start().await;
    let original = survivor.create_cat(SHELL_NAME).await;
    let original_runtime = original.runtime.as_ref().unwrap();
    let worker = crate::ProcessIdentity {
        pid: original_runtime.worker_pid,
        start_time: original_runtime.worker_start_time,
    };
    let shell = crate::ProcessIdentity {
        pid: original_runtime.child_pid,
        start_time: original_runtime.child_start_time,
    };
    assert!(worker.matches() && shell.matches());
    let serving = Installation::new(Some(survivor.home.path()));
    let connection = yolo_tui_local_startup::prepare(&serving.profile, &serving.tui)
        .await
        .unwrap();
    let services = Services::capture(&serving.profile.host_home, &connection).await;
    for signal in [libc::SIGINT, libc::SIGQUIT] {
        let pending = Installation::pending_bootstrap();
        process::cancel(&pending, signal).await;
        let (host, hub) = Services::inspect(&serving.profile.host_home, &connection).await;
        assert_eq!(host, services.host);
        assert_eq!(hub, services.hub);
        assert!(host.matches() && hub.matches() && worker.matches() && shell.matches());
        let current = survivor
            .client()
            .get_session(original.manifest.terminal_id)
            .await
            .unwrap();
        let runtime = current.runtime.as_ref().unwrap();
        assert_eq!(runtime.runtime_epoch, original_runtime.runtime_epoch);
        assert!(matches!(
            runtime.status,
            yolo_runtime::model::TerminalRuntimeStatus::Running
                | yolo_runtime::model::TerminalRuntimeStatus::OutputBlocked
        ));
        assert!(!current.manifest.archived);
        pending.remove_after_services_stopped();
    }
    services.stop().await;
    serving.remove_after_services_stopped();
}
