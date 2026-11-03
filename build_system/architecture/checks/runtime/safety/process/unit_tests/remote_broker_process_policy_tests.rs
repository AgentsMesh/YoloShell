use yolo_architecture_test_support::Fixture;

const BROKER_SOURCE: &str = "core/crates/transport/remote/broker/src/server/server.rs";

fn messages(source: &str) -> Vec<String> {
    let fixture = Fixture::new();
    fixture.write(BROKER_SOURCE, source);
    crate::inspect(fixture.root())
        .unwrap()
        .into_iter()
        .map(|violation| violation.to_string())
        .collect()
}

#[test]
fn accepts_task_threads_and_exact_boot_observation() {
    let source = r#"
fn serve() {
    tokio::spawn(async {});
    std::thread::spawn(|| {});
    let boot = crate::process::boot_id();
    let _ = crate::process::boot_identity_relation(&boot, &boot);
}
"#;
    assert!(messages(source).is_empty());
}

#[test]
fn rejects_process_command_and_pty_aliases() {
    for source in [
        "use std::process::Command as Child; fn run() {}",
        "use tokio::process::{Command as Child}; fn run() {}",
        "fn run() { std::process::Command::new(\"sh\"); }",
        "use portable_pty::CommandBuilder as Child; fn run() {}",
        "fn run(slave: Slave, child: Child) { slave.spawn_command(child); }",
    ] {
        assert_eq!(messages(source).len(), 1, "{source}");
    }
}

#[test]
fn rejects_escape_primitives_imports_and_aliases() {
    for source in [
        "use libc::fork as launch; fn run() {}",
        "use libc::{clone as launch}; fn run() {}",
        "fn run() { let launch = libc::clone; }",
        "fn run() { unsafe { libc::setsid(); } }",
        "fn run() { unsafe { libc::posix_spawnp(); } }",
        "fn run() { unsafe { libc::syscall(libc::SYS_clone3); } }",
        "extern \"C\" { #[link_name = \"fork\"] fn launch(); }",
        "fn run() { unsafe { dlsym(handle, symbol); } }",
        "fn run() { system(command); }",
    ] {
        assert!(!messages(source).is_empty(), "{source}");
    }
}

#[test]
fn comments_strings_and_async_spawn_do_not_spoof_process_creation() {
    let source = r#"
fn run(tasks: &mut JoinSet<()>) {
    // std::process::Command::new("sh"); libc::fork();
    let evidence = "libc::setsid and posix_spawn are prohibited";
    tasks.spawn(async move { consume(evidence).await });
}
"#;
    assert!(messages(source).is_empty());
}

#[test]
fn test_only_hooks_may_create_a_fixture_child() {
    let source = r#"
#[cfg(any(test, feature = "adversarial-test-hooks"))]
fn fixture() {
    std::process::Command::new("true");
}
"#;
    assert!(messages(source).is_empty());
}

#[test]
fn similarly_named_code_outside_the_broker_is_not_owned_by_this_rule() {
    let fixture = Fixture::new();
    fixture.write(
        "core/crates/application/broker/supervision/src/supervisor_process.rs",
        "fn launch() { std::process::Command::new(\"self\"); }",
    );
    assert!(crate::inspect(fixture.root()).unwrap().is_empty());
}
