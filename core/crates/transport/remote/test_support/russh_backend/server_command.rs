pub(super) fn runtime(command: &[u8]) -> bool {
    use super::constants::{MEMORY_HOME, MEMORY_RUNTIME};
    use crate::remote::constants::{
        BRIDGE_COMMAND, HOME_ARGUMENT, SHELL_SEPARATOR, STDIO_ARGUMENT,
    };
    let expected = std::iter::once(MEMORY_RUNTIME)
        .chain(yolo_launch_contract::ProcessRole::Host.arguments())
        .chain([BRIDGE_COMMAND, STDIO_ARGUMENT, HOME_ARGUMENT, MEMORY_HOME])
        .map(|argument| format!("'{argument}'"))
        .collect::<Vec<_>>()
        .join(SHELL_SEPARATOR);
    command == expected.as_bytes()
}

pub(super) fn contains(value: &[u8], needle: &[u8]) -> bool {
    value.windows(needle.len()).any(|window| window == needle)
}

pub(super) fn output(filesystem: &super::memory_fs::MemoryFilesystem, command: &[u8]) -> Vec<u8> {
    use super::constants::*;
    use crate::remote::constants::{
        ARCHITECTURE_PROGRAM, PLATFORM_ARGUMENT, RUNTIME_VERSION_ARGUMENT,
    };
    if contains(command, ARCHITECTURE_PROGRAM.as_bytes()) {
        if contains(command, PLATFORM_ARGUMENT.as_bytes()) {
            MEMORY_PLATFORM.as_bytes().to_vec()
        } else {
            MEMORY_ARCHITECTURE.as_bytes().to_vec()
        }
    } else if contains(command, RUNTIME_VERSION_ARGUMENT.as_bytes()) {
        format!("{RUNTIME_VERSION_OUTPUT_PREFIX}{}\n", filesystem.version()).into_bytes()
    } else if contains(command, LARGE_PROGRAM.as_bytes()) {
        vec![b'x'; LARGE_OUTPUT_BYTES]
    } else {
        COMMAND_STDOUT.to_vec()
    }
}
