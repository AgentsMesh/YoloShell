"""One executable and explicit process roles, shared by Rust entries."""

YOLO_EXECUTABLE = "yolo"
INTERNAL_COMMAND = "internal"
PROCESS_ROLES = [
    ("HOST_ROLE", "host", "host"),
    ("HUB_ROLE", "hub", "hub"),
    ("ATTACH_ROLE", "attach", "attach"),
    ("REMOTE_ROLE", "remote", "remote"),
    ("HUB_ATTACH_ROLE", "hubAttach", "hub-attach"),
    ("FILE_HELPER_ROLE", "fileHelper", "file-helper"),
]

def _rust_impl(ctx):
    values = [("EXECUTABLE_NAME", YOLO_EXECUTABLE), ("INTERNAL_COMMAND", INTERNAL_COMMAND)]
    values += [(rust_name, value) for rust_name, _, value in PROCESS_ROLES]
    values += [
        (rust_name + "_DISPLAY", "%s %s %s" % (YOLO_EXECUTABLE, INTERNAL_COMMAND, value))
        for rust_name, _, value in PROCESS_ROLES
    ]
    ctx.actions.write(ctx.outputs.out, "".join([
        'pub const %s: &str = "%s";\n' % (name, value)
        for name, value in values
    ]))

process_role_rust_contract = rule(
    implementation = _rust_impl,
    outputs = {"out": "src/generated/%{name}.rs"},
)
