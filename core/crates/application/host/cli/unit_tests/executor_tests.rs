use super::*;
use tokio::runtime::RuntimeFlavor;

const TEST_HOME: &str = "/tmp/yolo-executor-policy";

#[test]
fn persistent_custody_services_keep_multi_thread_execution() {
    let home = std::path::PathBuf::from(TEST_HOME);
    let terminal_id = uuid::Uuid::new_v4();
    for (command, threads) in [
        (
            Command::Serve {
                home: home.clone(),
                socket: None,
                detached_session: false,
                startup_guard: false,
            },
            HOST_SCHEDULER_THREADS,
        ),
        (
            Command::SessionWorker {
                home: home.clone(),
                terminal_id,
            },
            SESSION_SCHEDULER_THREADS,
        ),
        (
            Command::CustodyGuardian { home, terminal_id },
            SESSION_SCHEDULER_THREADS,
        ),
    ] {
        let runtime = build(&command).unwrap();
        assert_eq!(
            runtime.handle().runtime_flavor(),
            RuntimeFlavor::MultiThread
        );
        assert_eq!(runtime.metrics().num_workers(), threads);
        runtime.block_on(async {
            tokio::spawn(async { tokio::task::yield_now().await })
                .await
                .unwrap();
            assert_eq!(
                tokio::task::spawn_blocking(move || threads).await.unwrap(),
                threads
            );
            tokio::time::sleep(std::time::Duration::ZERO).await;
        });
    }
}

#[test]
fn helpers_keep_async_io_without_starting_a_scheduler_pool() {
    let home = std::path::PathBuf::from(TEST_HOME);
    for command in [
        Command::BootstrapChild,
        Command::ProviderHook {
            provider: yolo_claude_provider::INSTALL_TOKEN.into(),
            home: home.clone(),
        },
        Command::Bridge {
            home: home.clone(),
            socket: None,
            stdio: true,
        },
        Command::Ctl(crate::CtlOptions {
            home: Some(home),
            socket: None,
            existing_socket: false,
            existing_host: true,
            command: crate::CtlCommand::Status,
        }),
    ] {
        let runtime = build(&command).unwrap();
        assert_eq!(
            runtime.handle().runtime_flavor(),
            RuntimeFlavor::CurrentThread
        );
        runtime.block_on(async {
            tokio::spawn(async { tokio::task::yield_now().await })
                .await
                .unwrap();
            tokio::time::sleep(std::time::Duration::ZERO).await;
        });
    }
}
