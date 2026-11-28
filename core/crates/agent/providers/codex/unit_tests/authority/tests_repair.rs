use super::*;

const FOREIGN_PROVIDER_START: u64 = PROVIDER_START + 1;

#[test]
fn confirmed_exit_repairs_a_missing_binding_exactly() {
    let fixture = Fixture::new();
    let exited = confirmed_exit(&fixture);
    fixture.publish(&exited).unwrap();
    fixture
        .namespace
        .remove_file(&fixture.binding_path)
        .unwrap();

    repair(&fixture).unwrap();

    assert_binding(&fixture, &exited.binding);
}

#[test]
fn confirmed_exit_replaces_a_stale_running_binding_exactly() {
    let fixture = Fixture::new();
    let exited = confirmed_exit(&fixture);
    fixture.publish(&exited).unwrap();
    let running = running_binding(&exited);
    fixture
        .namespace
        .write_json(&fixture.binding_path, &running)
        .unwrap();

    repair(&fixture).unwrap();

    assert_binding(&fixture, &exited.binding);
}

#[test]
fn authority_change_before_binding_commit_aborts_the_repair() {
    let fixture = Fixture::new();
    let exited = confirmed_exit(&fixture);
    fixture.publish(&exited).unwrap();
    let running = running_binding(&exited);
    fixture
        .namespace
        .write_json(&fixture.binding_path, &running)
        .unwrap();
    let replacement = exited.unavailable().unwrap();
    let authority_path = fixture
        .binding_path
        .parent()
        .unwrap()
        .join(AUTHORITY_FILE_NAME);

    let result = fixture
        .store()
        .repair_active_binding_in_namespace_with_commit_fence(
            &fixture.namespace,
            &fixture.binding_path,
            fixture.terminal_id,
            fixture.runtime,
            |_| Ok(()),
            || fixture.namespace.write_json(&authority_path, &replacement),
        );

    assert!(result.is_err());
    assert_binding(&fixture, &running);
}

#[test]
fn non_exit_authorities_never_replace_the_binding() {
    for status in [
        CodexAuthorityStatus::Active,
        CodexAuthorityStatus::Unavailable,
    ] {
        let fixture = Fixture::new();
        let authority = fixture.authority(Generation::INITIAL.0, status);
        fixture.publish(&authority).unwrap();
        let foreign = confirmed_exit(&fixture).binding;
        fixture
            .namespace
            .write_json(&fixture.binding_path, &foreign)
            .unwrap();

        repair(&fixture).unwrap();

        assert_binding(&fixture, &foreign);
    }
}

#[test]
fn another_provider_binding_is_never_replaced() {
    let fixture = Fixture::new();
    let exited = confirmed_exit(&fixture);
    fixture.publish(&exited).unwrap();
    let mut foreign = running_binding(&exited);
    foreign.kind = AgentKind::ClaudeCode;
    fixture
        .namespace
        .write_json(&fixture.binding_path, &foreign)
        .unwrap();

    repair(&fixture).unwrap();

    assert_binding(&fixture, &foreign);
}

#[test]
fn another_codex_session_binding_is_never_replaced() {
    let fixture = Fixture::new();
    let exited = confirmed_exit(&fixture);
    fixture.publish(&exited).unwrap();
    let mut foreign = running_binding(&exited);
    foreign.provider_session_id = Uuid::new_v4().hyphenated().to_string();
    fixture
        .namespace
        .write_json(&fixture.binding_path, &foreign)
        .unwrap();

    repair(&fixture).unwrap();

    assert_binding(&fixture, &foreign);
}

#[test]
fn another_provider_process_binding_is_never_replaced() {
    let fixture = Fixture::new();
    let exited = confirmed_exit(&fixture);
    fixture.publish(&exited).unwrap();
    let mut foreign = running_binding(&exited);
    foreign.process_start_time = Some(FOREIGN_PROVIDER_START);
    fixture
        .namespace
        .write_json(&fixture.binding_path, &foreign)
        .unwrap();

    repair(&fixture).unwrap();

    assert_binding(&fixture, &foreign);
}

#[test]
fn another_yolo_session_authority_cannot_restore_a_missing_binding() {
    let fixture = Fixture::new();
    let mut exited = confirmed_exit(&fixture);
    exited.binding.terminal_id = Uuid::new_v4();
    let exited = exited.with_verified_resume_identity(TEST_RESUME_BOOT).unwrap();
    fixture.publish(&exited).unwrap();
    fixture
        .namespace
        .remove_file(&fixture.binding_path)
        .unwrap();

    repair(&fixture).unwrap();

    assert!(
        fixture
            .namespace
            .file_is_missing(&fixture.binding_path)
            .unwrap()
    );
}

fn confirmed_exit(fixture: &Fixture) -> CodexBindingAuthority {
    let mut authority = fixture.verified_authority(Generation::INITIAL.0);
    authority.binding.state = AgentState::Exited;
    authority.binding.process_pid = None;
    authority.binding.process_start_time = None;
    authority
}

fn running_binding(authority: &CodexBindingAuthority) -> AgentBinding {
    let mut binding = authority.binding.clone();
    binding.state = AgentState::Running;
    binding.process_pid = Some(PROVIDER_PID);
    binding.process_start_time = Some(PROVIDER_START);
    binding
}

fn repair(fixture: &Fixture) -> anyhow::Result<()> {
    fixture
        .store()
        .repair_active_binding_in_namespace_with_commit_fence(
            &fixture.namespace,
            &fixture.binding_path,
            fixture.terminal_id,
            fixture.runtime,
            |_| Ok(()),
            || Ok(()),
        )
}

fn assert_binding(fixture: &Fixture, expected: &AgentBinding) {
    let actual = fixture
        .namespace
        .read_json::<AgentBinding>(&fixture.binding_path)
        .unwrap();
    assert_eq!(&actual, expected);
}
