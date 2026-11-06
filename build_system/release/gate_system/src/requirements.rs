pub(crate) use crate::gate_command::coverage_report_path;
use yolo_release_contracts::*;

const PACKAGE_ARGUMENTS: &[&str] = &[
    BAZEL_BUILD_VERB,
    BAZEL_MACOS_CONFIG_ARGUMENT,
    BAZEL_OPTIMIZED_ARGUMENT,
    SERIAL_JOBS_ARGUMENT,
    PACKAGE_TARGET,
];
const UNIT_ARGUMENTS: &[&str] = &[
    BAZEL_TEST_VERB,
    SERIAL_JOBS_ARGUMENT,
    NO_CACHE_ARGUMENT,
    UNIT_TARGET,
];
const E2E_ARGUMENTS: &[&str] = &[
    BAZEL_TEST_VERB,
    BAZEL_MACOS_CONFIG_ARGUMENT,
    BAZEL_OPTIMIZED_ARGUMENT,
    SERIAL_JOBS_ARGUMENT,
    NO_CACHE_ARGUMENT,
    E2E_TARGET,
];
const RUST_COVERAGE_ARGUMENTS: &[&str] = &[
    BAZEL_COVERAGE_VERB,
    BAZEL_RUST_CONFIG_ARGUMENT,
    SERIAL_JOBS_ARGUMENT,
    NO_CACHE_ARGUMENT,
    RUST_TESTS_TARGET,
];
const RUST_GATE_ARGUMENTS: &[&str] = &[
    BAZEL_RUN_VERB,
    SERIAL_JOBS_ARGUMENT,
    RUST_COVERAGE_TARGET,
    BAZEL_ARGUMENT_SEPARATOR,
];
const SWIFT_COVERAGE_ARGUMENTS: &[&str] = &[
    BAZEL_COVERAGE_VERB,
    BAZEL_MACOS_CONFIG_ARGUMENT,
    SERIAL_JOBS_ARGUMENT,
    NO_CACHE_ARGUMENT,
    SWIFT_COVERAGE_TARGET,
];
const UNIT_COMMANDS: &[GateCommand] = &[GateCommand::fixed(UNIT_ARGUMENTS)];
const E2E_COMMANDS: &[GateCommand] = &[GateCommand::fixed(E2E_ARGUMENTS)];
const RUST_COVERAGE_COMMANDS: &[GateCommand] = &[
    GateCommand::coverage_producer(RUST_COVERAGE_ARGUMENTS),
    GateCommand::coverage_consumer(RUST_GATE_ARGUMENTS),
];
const SWIFT_COVERAGE_COMMANDS: &[GateCommand] = &[GateCommand::fixed(SWIFT_COVERAGE_ARGUMENTS)];

pub const LINUX_AMD64_PROFILE: LinuxGateProfile = LinuxGateProfile {
    architecture: LINUX_AMD64_RUNTIME_ARCHITECTURE,
    docker_architecture: LINUX_AMD64_DOCKER_ARCHITECTURE,
    docker_platform: LINUX_AMD64_PLATFORM,
    image: LINUX_AMD64_IMAGE,
    bazel_sha256: LINUX_AMD64_BAZEL_SHA256,
    bazel_environment: LINUX_AMD64_BAZEL_ENVIRONMENT,
    elf_machine: ELF_MACHINE_X86_64,
    bundle_root: LINUX_AMD64_BUNDLE_ROOT,
};
pub const LINUX_ARM64_PROFILE: LinuxGateProfile = LinuxGateProfile {
    architecture: LINUX_ARM64_RUNTIME_ARCHITECTURE,
    docker_architecture: LINUX_ARM64_DOCKER_ARCHITECTURE,
    docker_platform: LINUX_ARM64_PLATFORM,
    image: LINUX_ARM64_IMAGE,
    bazel_sha256: LINUX_ARM64_BAZEL_SHA256,
    bazel_environment: LINUX_ARM64_BAZEL_ENVIRONMENT,
    elf_machine: ELF_MACHINE_AARCH64,
    bundle_root: LINUX_ARM64_BUNDLE_ROOT,
};

pub const PACKAGE_COMMAND: GateCommand = GateCommand::fixed(PACKAGE_ARGUMENTS);
pub const REQUIRED_GATES: [GateRequirement; 6] = [
    GateRequirement::linux(LINUX_AMD64_GATE, LINUX_AMD64_PROFILE),
    GateRequirement::linux(LINUX_ARM64_GATE, LINUX_ARM64_PROFILE),
    GateRequirement::bazel(UNIT_GATE, UNIT_TARGET, UNIT_COMMANDS),
    GateRequirement::bazel(E2E_GATE, E2E_TARGET, E2E_COMMANDS),
    GateRequirement::bazel(
        RUST_COVERAGE_GATE,
        RUST_COVERAGE_TARGET,
        RUST_COVERAGE_COMMANDS,
    ),
    GateRequirement::bazel(
        SWIFT_COVERAGE_GATE,
        SWIFT_COVERAGE_TARGET,
        SWIFT_COVERAGE_COMMANDS,
    ),
];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct GateCommand {
    pub(crate) arguments: &'static [&'static str],
    pub(crate) coverage_action: CoverageAction,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum CoverageAction {
    None,
    Produce,
    Consume,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct GateRequirement {
    pub name: &'static str,
    pub target: &'static str,
    pub kind: GateKind,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GateKind {
    Bazel(&'static [GateCommand]),
    Linux(LinuxGateProfile),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct LinuxGateProfile {
    pub architecture: &'static str,
    pub docker_architecture: &'static str,
    pub docker_platform: &'static str,
    pub image: &'static str,
    pub bazel_sha256: &'static str,
    pub bazel_environment: &'static str,
    pub elf_machine: u16,
    pub bundle_root: &'static str,
}
