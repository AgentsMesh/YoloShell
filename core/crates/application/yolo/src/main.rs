//! Composition root only: argument selection, role dispatch and exit reporting.
mod dispatch;
mod exit;

fn main() -> std::process::ExitCode {
    let invocation = match yolo_launch_contract::Invocation::parse(std::env::args_os()) {
        Ok(invocation) => invocation,
        Err(error) => return exit::usage(error),
    };
    let role = invocation.role;
    exit::finish(role, dispatch::run(invocation))
}
