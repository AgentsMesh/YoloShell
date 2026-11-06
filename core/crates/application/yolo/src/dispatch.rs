use yolo_launch_contract::{Invocation, ProcessRole};

pub fn run(invocation: Invocation) -> anyhow::Result<()> {
    let Invocation { role, arguments } = invocation;
    match role {
        None => {
            let host = yolo_builtin_client_host::BuiltinClientHost::builtin(vec![
                yolo_tui_descriptor::tui_descriptor(),
            ])?;
            yolo_builtin_client_host::launch(&host, yolo_tui_cli::TuiBuiltinClient, arguments)
        }
        Some(ProcessRole::Host) => yolo_host_cli::run_from(arguments),
        Some(ProcessRole::Hub) => yolo_hub_cli::run_from(arguments),
        Some(ProcessRole::Attach) => yolo_attach_cli::run_from(arguments),
        Some(ProcessRole::Remote) => yolo_remote_cli::run_from(arguments),
        Some(ProcessRole::HubAttach) => yolo_hub_attach_cli::run_from(arguments),
        Some(ProcessRole::FileHelper) => yolo_hub_file_helper::run_from(arguments),
    }
}
