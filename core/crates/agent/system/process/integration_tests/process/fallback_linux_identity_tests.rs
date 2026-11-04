use super::{process_identity, process_row_from_identity};
use crate::source::fallback::SystemApi;
use crate::source::fallback_observation::observe_process;

#[test]
fn observed_linux_executable_is_the_pinned_file_path_not_the_stat_command_name() {
    let identity = process_identity(std::process::id()).unwrap();
    let expected =
        std::fs::read_link(crate::process::procfs::executable_path(identity.pid)).unwrap();
    assert!(expected.is_absolute());
    assert_eq!(identity.executable, expected.to_str().unwrap());
    assert_eq!(identity.executable_file.path, expected);

    let observed = observe_process(&SystemApi, process_row_from_identity(identity), &[])
        .expect("current test process must retain its fenced executable identity");
    assert_eq!(observed.executable, expected.to_str().unwrap());
}
