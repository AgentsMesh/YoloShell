
load(
    "//build_system/architecture:topology.bzl",
    "SOCKET_ROOT_GUARD",
)
_FIXTURE_EXECUTABLE_ENVIRONMENT = "YOLO_SOCKET_ROOT_GUARD_PROCESS_TEST_BINARY"
_GUARD_BINARY_ENVIRONMENT = "YOLO_SOCKET_ROOT_GUARD_BINARY"
_COVERAGE_OBJECTS_ENVIRONMENT = "YOLO_ADDITIONAL_COVERAGE_OBJECTS"

def _process_test_impl(ctx):
    binary = ctx.executable.binary
    guard = ctx.executable._guard
    executable = ctx.actions.declare_file(ctx.label.name)
    ctx.actions.symlink(
        output = executable,
        target_file = binary,
        is_executable = True,
    )
    binary_environment = ctx.attr.binary[RunEnvironmentInfo]
    environment = dict(binary_environment.environment)
    environment[_FIXTURE_EXECUTABLE_ENVIRONMENT] = binary.short_path
    environment[_GUARD_BINARY_ENVIRONMENT] = guard.short_path
    environment[_COVERAGE_OBJECTS_ENVIRONMENT] = json.encode([guard.short_path])
    runfiles = ctx.runfiles(files = [binary, guard]).merge(
        ctx.attr.binary[DefaultInfo].default_runfiles,
    ).merge(
        ctx.attr._guard[DefaultInfo].default_runfiles,
    )
    return [
        DefaultInfo(executable = executable, runfiles = runfiles),
        RunEnvironmentInfo(
            environment = environment,
            inherited_environment = binary_environment.inherited_environment,
        ),
        coverage_common.instrumented_files_info(
            ctx,
            dependency_attributes = ["binary", "_guard"],
        ),
    ]

socket_root_guard_process_test = rule(
    implementation = _process_test_impl,
    test = True,
    attrs = {
        "binary": attr.label(executable = True, cfg = "target", mandatory = True),
        "_collect_cc_coverage": attr.label(
            default = Label("@rules_rust//util/collect_coverage"),
            executable = True,
            cfg = "exec",
        ),
        "_guard": attr.label(
            default = Label(SOCKET_ROOT_GUARD),
            executable = True,
            cfg = "target",
        ),
        "_lcov_merger": attr.label(
            default = configuration_field(fragment = "coverage", name = "output_generator"),
            executable = True,
            cfg = "exec",
        ),
    },
)
