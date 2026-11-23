use super::*;
use std::path::Path;

const PADDED_AGENT_SESSION_ID: &str = " agent-session ";

#[test]
fn core_validation_rejects_nonblank_padded_session_identity() {
    let (_directory, context, mut binding, _root) = fixture();
    binding.provider_session_id = PADDED_AGENT_SESSION_ID.into();

    rejected(
        &binding,
        &context,
        crate::constants::message::AGENT_SESSION_ID,
    );
}

#[test]
fn core_validation_rejects_every_identity_and_lifecycle_boundary() {
    let (_directory, context, binding, _root) = fixture();
    validate_core(&binding, &context).unwrap();
    let mut invalid = binding.clone();
    invalid.provider_session_id = " \t".into();
    rejected(
        &invalid,
        &context,
        crate::constants::message::AGENT_SESSION_ID,
    );
    invalid = binding.clone();
    invalid.detected_at_ms = -1;
    rejected(&invalid, &context, crate::constants::message::TIMESTAMPS);
    invalid = binding.clone();
    invalid.updated_at_ms = invalid.detected_at_ms - 1;
    rejected(&invalid, &context, crate::constants::message::TIMESTAMPS);
    invalid = binding.clone();
    invalid.process_pid = Some(0);
    rejected(&invalid, &context, crate::constants::message::PROCESS_PID);
    invalid = binding.clone();
    invalid.process_pid = None;
    rejected(&invalid, &context, crate::constants::message::PROCESS_PID);
    invalid = binding.clone();
    invalid.state = AgentState::Exited;
    rejected(&invalid, &context, crate::constants::message::PROCESS_PID);
    invalid.process_pid = None;
    invalid.process_start_time = None;
    validate_core(&invalid, &context).unwrap();

    invalid = binding.clone();
    invalid.process_start_time = None;
    rejected(
        &invalid,
        &context,
        crate::constants::message::PROCESS_START_TIME,
    );
    invalid.process_start_time = Some(0);
    rejected(
        &invalid,
        &context,
        crate::constants::message::PROCESS_START_TIME,
    );
}

#[test]
fn adapter_validation_covers_executable_and_evidence_boundaries() {
    let (directory, context, binding, root) = fixture();
    let names = [TEST_EXECUTABLE];
    validate_adapter_binding(
        &binding,
        &context,
        AgentKind::Codex,
        &names,
        context.data_home(&AgentKind::Codex).unwrap(),
        &root,
    )
    .unwrap();
    let mut node = binding.clone();
    node.executable = "node".into();
    validate_adapter_binding(
        &node,
        &context,
        AgentKind::Codex,
        &names,
        context.data_home(&AgentKind::Codex).unwrap(),
        &root,
    )
    .unwrap();
    let cases = [
        (
            crate::constants::message::EXECUTABLE,
            directory.path().join("outside.jsonl"),
            "sh",
            binding.data_home.clone(),
        ),
        (
            crate::constants::message::EVIDENCE_PATH,
            directory.path().join("outside.jsonl"),
            names[0],
            binding.data_home.clone(),
        ),
        (
            crate::constants::message::DATA_HOME,
            PathBuf::from(&binding.evidence_path),
            names[0],
            String::new(),
        ),
    ];
    for (expected, path, executable, data_home) in cases {
        let mut invalid = binding.clone();
        invalid.executable = executable.into();
        invalid.evidence_path = path.display().to_string();
        invalid.data_home = data_home;
        let error = validate_adapter_binding(
            &invalid,
            &context,
            AgentKind::Codex,
            &names,
            context.data_home(&AgentKind::Codex).unwrap(),
            &root,
        )
        .unwrap_err();
        assert!(error.to_string().contains(expected), "{error:#}");
    }
    let mut wrong_extension = binding.clone();
    wrong_extension.evidence_path = root.join("evidence.json").display().to_string();
    assert!(
        validate_adapter_binding(
            &wrong_extension,
            &context,
            AgentKind::Codex,
            &names,
            context.data_home(&AgentKind::Codex).unwrap(),
            &root,
        )
        .is_err()
    );
    let mut relative_cwd = binding;
    relative_cwd.cwd = "relative".into();
    assert!(
        validate_adapter_binding(
            &relative_cwd,
            &context,
            AgentKind::Codex,
            &names,
            context.data_home(&AgentKind::Codex).unwrap(),
            &root,
        )
        .is_err()
    );
}

#[test]
fn path_containment_handles_canonical_fallback_and_parent_components() {
    let (directory, _context, binding, root) = fixture();
    assert!(path_is_within(Path::new(&binding.evidence_path), &root));
    assert!(!path_is_within(
        &directory.path().join("outside.jsonl"),
        &root
    ));
    assert!(path_is_within(&root.join("missing.jsonl"), &root));
    assert!(!path_is_within(Path::new("relative/missing.jsonl"), &root));
    assert!(!path_is_within(&root.join("../escape.jsonl"), &root));
}
