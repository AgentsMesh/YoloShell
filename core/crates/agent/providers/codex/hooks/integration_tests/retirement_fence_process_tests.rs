use super::retirement_fence_process_support::{ProcessRaceContext, ProcessRaceRole};
use super::retirement_test_constants::*;
use super::retirement_test_support::RetirementFixture;
use crate::codex_hooks::CodexRetirementWriterFence;
use std::path::{Path, PathBuf};
use std::process::{Child, Command};
use std::time::Instant;

#[test]
fn producer_wins_real_process_writer_fence_before_sealer() {
    let fixture = RetirementFixture::new();
    let context = serde_json::to_string(&ProcessRaceContext::capture(&fixture)).unwrap();
    let acquired = fixture.home().join(PRODUCER_ACQUIRED_FILE);
    let contended = fixture.home().join(SEALER_CONTENDED_FILE);
    let release = fixture.home().join(PRODUCER_RELEASE_FILE);
    let mut producer = launch(
        &context,
        PRODUCER_ROLE,
        EXPECTED_SUCCESS,
        Some((&acquired, &release)),
        None,
    );
    wait_for_path(&acquired, &mut producer);
    let mut sealer = launch(
        &context,
        SEALER_ROLE,
        EXPECTED_SUCCESS,
        None,
        Some(&contended),
    );
    wait_for_both(&contended, &mut producer, &mut sealer);
    signal(&release);

    assert!(producer.wait().unwrap().success());
    assert!(sealer.wait().unwrap().success());
    assert_eq!(
        fixture.intake().scan().unwrap().occurrences.len(),
        PRODUCER_WIN_OCCURRENCE_COUNT
    );
    assert!(fixture.seal_path().exists());
}

#[test]
fn sealer_wins_real_process_writer_fence_before_producer() {
    let fixture = RetirementFixture::new();
    let context = serde_json::to_string(&ProcessRaceContext::capture(&fixture)).unwrap();
    let acquired = fixture.home().join(SEALER_ACQUIRED_FILE);
    let contended = fixture.home().join(PRODUCER_CONTENDED_FILE);
    let release = fixture.home().join(SEALER_RELEASE_FILE);
    let mut sealer = launch(
        &context,
        SEALER_ROLE,
        EXPECTED_SUCCESS,
        Some((&acquired, &release)),
        None,
    );
    wait_for_path(&acquired, &mut sealer);
    let mut producer = launch(
        &context,
        PRODUCER_ROLE,
        EXPECTED_FAILURE,
        None,
        Some(&contended),
    );
    wait_for_both(&contended, &mut sealer, &mut producer);
    signal(&release);

    assert!(sealer.wait().unwrap().success());
    assert!(producer.wait().unwrap().success());
    assert!(fixture.intake().scan().unwrap().occurrences.is_empty());
    assert!(fixture.seal_path().exists());
}

#[test]
fn codex_retirement_writer_fence_subprocess_helper() {
    if std::env::var_os(MODE_ENV).as_deref() != Some(HELPER_MODE.as_ref()) {
        return;
    }
    install_hold_hook();
    if let Some(path) = env_path(CONTENDED_ENV) {
        crate::paths::test_support::set_private_file_wait_contention_hook(move || signal(&path));
    }
    let context: ProcessRaceContext =
        serde_json::from_str(&std::env::var(CONTEXT_ENV).unwrap()).unwrap();
    let result = context.run(role());
    let expected = std::env::var(EXPECTED_ENV).unwrap();
    assert_eq!(result.is_ok(), expected == EXPECTED_SUCCESS, "{result:?}");
}

fn install_hold_hook() {
    let Some(acquired) = env_path(ACQUIRED_ENV) else {
        return;
    };
    let release = env_path(RELEASE_ENV).unwrap();
    CodexRetirementWriterFence::set_after_acquire_hook(move || {
        signal(&acquired);
        wait_until(&release, || true);
    });
}

fn launch(
    context: &str,
    role: &str,
    expected: &str,
    hold: Option<(&Path, &Path)>,
    contended: Option<&Path>,
) -> Child {
    let mut command = Command::new(std::env::current_exe().unwrap());
    command
        .arg(HELPER_FILTER)
        .arg(NOCAPTURE_ARGUMENT)
        .env(MODE_ENV, HELPER_MODE)
        .env(CONTEXT_ENV, context)
        .env(EXPECTED_ENV, expected)
        .env(ROLE_ENV, role);
    if let Some((acquired, release)) = hold {
        command
            .env(ACQUIRED_ENV, acquired)
            .env(RELEASE_ENV, release);
    }
    if let Some(path) = contended {
        command.env(CONTENDED_ENV, path);
    }
    crate::process::spawn_waitable_test_child(command)
}

fn role() -> ProcessRaceRole {
    match std::env::var(ROLE_ENV).unwrap().as_str() {
        PRODUCER_ROLE => ProcessRaceRole::Producer,
        SEALER_ROLE => ProcessRaceRole::Sealer,
        unexpected => panic!("{UNEXPECTED_ROLE}: {unexpected}"),
    }
}

fn env_path(name: &str) -> Option<PathBuf> {
    std::env::var_os(name).map(PathBuf::from)
}

fn signal(path: &Path) {
    std::fs::write(path, SIGNAL_BYTES).unwrap();
}

fn wait_for_path(path: &Path, child: &mut Child) {
    wait_until(path, || child.try_wait().unwrap().is_none());
}

fn wait_for_both(path: &Path, first: &mut Child, second: &mut Child) {
    wait_until(path, || {
        first.try_wait().unwrap().is_none() && second.try_wait().unwrap().is_none()
    });
}

fn wait_until(path: &Path, healthy: impl FnMut() -> bool) {
    let deadline = Instant::now() + HELPER_TIMEOUT;
    let mut healthy = healthy;
    while !path.exists() {
        assert!(healthy());
        assert!(Instant::now() < deadline);
        std::thread::sleep(POLL_INTERVAL);
    }
}
