use super::*;

#[test]
fn process_entries_are_resolved_beneath_the_selected_root() {
    let root = Path::new("/fixture-proc");
    assert_eq!(
        process_entry_at(root, 42, CMDLINE_ENTRY),
        root.join("42/cmdline")
    );
    assert_eq!(process_entry_at(root, 42, CWD_ENTRY), root.join("42/cwd"));
    assert_eq!(
        process_entry_at(root, 42, EXECUTABLE_ENTRY),
        root.join("42/exe")
    );
    assert_eq!(
        process_entry_at(root, 42, LINUX_PROC_ENVIRONMENT_ENTRY),
        root.join("42/environ")
    );
    assert_eq!(
        process_entry_at(root, 42, FILE_DESCRIPTORS_ENTRY),
        root.join("42/fd")
    );
    assert_eq!(process_entry_at(root, 42, STAT_ENTRY), root.join("42/stat"));
    assert_eq!(
        process_entry_at(root, 42, LINUX_PROC_STATUS_ENTRY),
        root.join("42/status")
    );
}
