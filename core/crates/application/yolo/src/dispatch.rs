use yolo_launch_contract::{Invocation, ProcessRole};

pub fn run(invocation: Invocation) -> anyhow::Result<()> {
    let arguments = invocation.arguments;
    match invocation.role {
        None if yolo_agent_cli::selects(&arguments) => yolo_agent_cli::run_from(arguments),
        None if yolo_conversation_cli::selects(&arguments) => {
            yolo_conversation_cli::run_from(arguments)
        }
        None if yolo_workspace_cli::selects(&arguments) => yolo_workspace_cli::run_from(arguments),
        None if yolo_protocol_cli::selects(&arguments) => yolo_protocol_cli::run_from(arguments),
        None if yolo_trusted_plugin_cli::selects(&arguments) => {
            yolo_trusted_plugin_cli::run_from(arguments)
        }
        None if yolo_web_cli::selects(&arguments) => yolo_web_cli::run_from(arguments),
        None if yolo_upgrade_cli::selects(&arguments) => yolo_upgrade_cli::run_from(arguments),
        None => {
            let host = yolo_builtin_client_host::BuiltinClientHost::builtin(
                yolo_builtin_catalog::builtin_client_descriptors(),
            )?;
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
