const ROOT_FIRST_SHELL_COMMAND_ARGUMENT: &str = "-c";
const ROOT_FIRST_SHELL_NAME: &str = "yolo-root-first";
const ROOT_FIRST_STAGE_ROOT_STARTED: &str = "root-started";
const ROOT_FIRST_STAGE_INTERMEDIATE_LAUNCHING: &str = "intermediate-launching";
const ROOT_FIRST_STAGE_INTERMEDIATE_RETURNED: &str = "intermediate-returned-before-ready";
const ROOT_FIRST_SHELL_POLL_SECONDS: &str = "0.01";
const ROOT_FIRST_MARKER_PARENT_DETAIL: &str = "root-first marker has a parent directory";
const ROOT_FIRST_SHELL_SCRIPT: &str = concat!(
    "marker_parent=${15}; poll_seconds=${16}; ",
    "trap '' HUP TERM; printf '%s' \"$9\" > \"${10}\"; ",
    "while [ ! -e \"$1\" ] && [ -d \"$marker_parent\" ]; do sleep \"$poll_seconds\"; done; ",
    "if [ ! -d \"$marker_parent\" ]; then exit; fi; ",
    "printf '%s' \"${11}\" > \"${10}\"; ",
    "(env \"$2=$3\" \"$4\" \"$6\" \"$5\" \"$7\" \"$8\"; ",
    "if [ ! -e \"${12}\" ]; then printf '%s' \"${13}\" > \"${10}\"; fi) & ",
    "while [ ! -e \"${12}\" ] && [ -d \"$marker_parent\" ]; do sleep \"$poll_seconds\"; done; ",
    "if [ ! -d \"$marker_parent\" ]; then exit; fi; ",
    "while [ ! -e \"${14}\" ] && [ -d \"$marker_parent\" ]; do sleep \"$poll_seconds\"; done",
);

pub(super) fn root_first_command(
    marker: &Path,
    capability: &str,
    leaf_pty: RootFirstLeafPtyMode,
) -> CommandBuilder {
    let launch_marker = marker.with_extension(ROOT_FIRST_LAUNCH_EXTENSION);
    let trigger = marker.with_extension(ROOT_FIRST_TRIGGER_EXTENSION);
    let intermediate_lock = marker.with_extension(ROOT_FIRST_INTERMEDIATE_LOCK_EXTENSION);
    let intermediate_ready = marker.with_extension(ROOT_FIRST_INTERMEDIATE_READY_EXTENSION);
    let intermediate_release = marker.with_extension(ROOT_FIRST_INTERMEDIATE_RELEASE_EXTENSION);
    let reopen_ready = marker.with_extension(ROOT_FIRST_REOPEN_READY_EXTENSION);
    let reopen_release = marker.with_extension(ROOT_FIRST_REOPEN_RELEASE_EXTENSION);
    let root_release = marker.with_extension(ROOT_FIRST_ROOT_RELEASE_EXTENSION);
    let marker_parent = marker.parent().expect(ROOT_FIRST_MARKER_PARENT_DETAIL);
    let mut command = CommandBuilder::new(REAL_PTY_SHELL);
    command.args([
        ROOT_FIRST_SHELL_COMMAND_ARGUMENT.into(),
        ROOT_FIRST_SHELL_SCRIPT.into(),
        ROOT_FIRST_SHELL_NAME.into(),
        trigger.into_os_string(),
        ROOT_FIRST_HELPER_ROLE_ENV.into(),
        ROOT_FIRST_INTERMEDIATE_ROLE.into(),
        std::env::current_exe().unwrap().into_os_string(),
        ROOT_FIRST_HELPER_TEST.into(),
        BOOTSTRAP_TEST_EXACT_ARGUMENT.into(),
        BOOTSTRAP_TEST_NOCAPTURE_ARGUMENT.into(),
        crate::constants::TEST_HARNESS_SINGLE_THREAD_ARGUMENT.into(),
        ROOT_FIRST_STAGE_ROOT_STARTED.into(),
        marker.as_os_str().to_owned(),
        ROOT_FIRST_STAGE_INTERMEDIATE_LAUNCHING.into(),
        intermediate_ready.as_os_str().to_owned(),
        ROOT_FIRST_STAGE_INTERMEDIATE_RETURNED.into(),
        root_release.into_os_string(),
        marker_parent.as_os_str().to_owned(),
        ROOT_FIRST_SHELL_POLL_SECONDS.into(),
    ]);
    command.env(ROOT_FIRST_HELPER_MARKER_ENV, marker);
    command.env(ROOT_FIRST_LAUNCH_MARKER_ENV, &launch_marker);
    command.env(ROOT_FIRST_INTERMEDIATE_LOCK_ENV, intermediate_lock);
    command.env(ROOT_FIRST_INTERMEDIATE_READY_ENV, intermediate_ready);
    command.env(ROOT_FIRST_INTERMEDIATE_RELEASE_ENV, intermediate_release);
    command.env(ROOT_FIRST_REOPEN_READY_ENV, reopen_ready);
    command.env(ROOT_FIRST_REOPEN_RELEASE_ENV, reopen_release);
    command.env(process::PTY_SESSION_CAPABILITY_ENV, capability);
    if leaf_pty == RootFirstLeafPtyMode::Closed {
        command.env(ROOT_FIRST_SKIP_REOPEN_ENV, ROOT_FIRST_SKIP_REOPEN_VALUE);
    }
    command
}
