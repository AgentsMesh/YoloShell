use super::*;

pub(super) fn assert_managed_codex_arguments(process: &yolo_runtime::agent::ObservedProcess) {
    let (executable, arguments) = process.argv.split_first().unwrap();
    assert_eq!(
        Path::new(executable)
            .file_name()
            .and_then(|name| name.to_str()),
        Some(yolo_runtime::agent::codex::constants::EXECUTABLE)
    );
    assert!(arguments.is_empty());
}
