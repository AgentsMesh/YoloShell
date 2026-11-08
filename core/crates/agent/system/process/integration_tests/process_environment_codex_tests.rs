#[test]
fn codex_admission_fields_fit_the_process_environment_budget() {
    let names = yolo_codex_hooks::SENSITIVE_ENVIRONMENT_NAMES;
    assert!(names.len() <= super::constants::MAX_PROCESS_ENVIRONMENT_KEYS);
    for name in names {
        assert!(!name.is_empty());
    }
}
