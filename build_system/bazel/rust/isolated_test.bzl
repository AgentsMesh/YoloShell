
load("@rules_rust//rust:defs.bzl", "rust_test")

_BINARY_SUFFIX = "_isolated_binary"
_RUNNER = "//core/crates/application/socket/guard:socket_root_guard"
_SOCKET_ENVIRONMENT = "YOLO_SOCKET_ROOT"
_TEST_EXECUTABLE_ENVIRONMENT = "YOLO_TEST_EXECUTABLE"
_WRAPPER_REQUIRED = "yolo-test-wrapper-required"
_COVERAGE_OBJECTS_ENVIRONMENT = "YOLO_ADDITIONAL_COVERAGE_OBJECTS"

def _isolated_test_impl(ctx):
    binary = ctx.executable.binary
    executable = ctx.actions.declare_file(ctx.label.name)
    ctx.actions.symlink(
        output = executable,
        target_file = ctx.executable._runner,
        is_executable = True,
    )
    binary_environment = ctx.attr.binary[RunEnvironmentInfo]
    environment = dict(binary_environment.environment)
    environment[_TEST_EXECUTABLE_ENVIRONMENT] = binary.short_path
    objects = []
    for target in ctx.attr.coverage_objects:
        object = target[DefaultInfo].files_to_run.executable
        if object == None or object in objects:
            fail("additional coverage objects must be distinct executables: " + str(target.label))
        objects.append(object)
    if objects:
        if _COVERAGE_OBJECTS_ENVIRONMENT in environment:
            fail("declare coverage_objects instead of overriding the coverage object environment")
        environment[_COVERAGE_OBJECTS_ENVIRONMENT] = json.encode([object.short_path for object in objects])
    runfiles = ctx.runfiles(files = [binary]).merge(
        ctx.attr.binary[DefaultInfo].default_runfiles,
    ).merge(
        ctx.attr._runner[DefaultInfo].default_runfiles,
    )
    for target in ctx.attr.coverage_objects:
        runfiles = runfiles.merge(target[DefaultInfo].default_runfiles)
    runfiles = runfiles.merge(ctx.runfiles(files = objects))
    return [
        DefaultInfo(executable = executable, runfiles = runfiles),
        RunEnvironmentInfo(
            environment = environment,
            inherited_environment = binary_environment.inherited_environment,
        ),
        coverage_common.instrumented_files_info(
            ctx,
            dependency_attributes = ["binary", "coverage_objects"],
        ),
    ]

_isolated_test = rule(
    implementation = _isolated_test_impl,
    test = True,
    attrs = {
        "binary": attr.label(executable = True, cfg = "target", mandatory = True),
        "coverage_objects": attr.label_list(cfg = "target"),
        "_collect_cc_coverage": attr.label(
            default = Label("@rules_rust//util/collect_coverage"),
            executable = True,
            cfg = "exec",
        ),
        "_lcov_merger": attr.label(
            default = configuration_field(fragment = "coverage", name = "output_generator"),
            executable = True,
            cfg = "exec",
        ),
        "_runner": attr.label(
            default = Label(_RUNNER),
            executable = True,
            cfg = "exec",
        ),
    },
)

def isolated_rust_test(
        name,
        data = [],
        coverage_objects = [],
        env = {},
        env_inherit = [],
        tags = [],
        timeout = "moderate",
        visibility = None,
        **kwargs):
    binary_name = name + _BINARY_SUFFIX
    binary_env = dict(env)
    binary_env[_SOCKET_ENVIRONMENT] = _WRAPPER_REQUIRED
    rust_test(
        name = binary_name,
        data = data + [object for object in coverage_objects if object not in data],
        env = binary_env,
        env_inherit = env_inherit,
        tags = ["manual"],
        testonly = True,
        timeout = timeout,
        visibility = visibility,
        **kwargs
    )
    _isolated_test(
        name = name,
        binary = ":" + binary_name,
        coverage_objects = coverage_objects,
        tags = tags,
        timeout = timeout,
        visibility = visibility,
    )
