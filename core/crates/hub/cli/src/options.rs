use crate::constants::*;
use clap::{ArgGroup, Parser};
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = PROGRAM, version, about = DESCRIPTION)]
#[command(group(ArgGroup::new(REMOTE_ACTION_GROUP).args([INSTALL_BUNDLE_FIELD, INSPECT_INSTALL_FIELD, REMOTE_BOOTSTRAP_FIELD, REMOTE_RECOVERY_FIELD]).multiple(false)))]
pub(crate) struct Options {
    #[arg(long = STATE_OPTION, required_unless_present_any = [INSTALL_BUNDLE_FIELD, INSPECT_INSTALL_FIELD, PEER_STDIO_FIELD])]
    pub state: Option<PathBuf>,
    #[arg(long = HOST_HOME_OPTION, required_unless_present_any = [INSTALL_BUNDLE_FIELD, INSPECT_INSTALL_FIELD, PEER_STDIO_FIELD])]
    pub host_home: Option<PathBuf>,
    #[arg(long = PEER_STDIO_OPTION, hide = true, conflicts_with_all = [BOOTSTRAP_FIELD, REMOTE_BOOTSTRAP_FIELD, REMOTE_RECOVERY_FIELD, INSTALL_BUNDLE_FIELD, INSPECT_INSTALL_FIELD, READY_FIELD])]
    pub peer_stdio: bool,
    #[arg(long = PEER_SOCKET_OPTION, hide = true, requires = PEER_STDIO_FIELD, value_name = "PATH")]
    pub peer_socket: Option<PathBuf>,
    #[arg(long = HOST_SOCKET_OPTION)]
    pub host_socket: Option<PathBuf>,
    #[arg(long = PEER_CONFIG_OPTION)]
    pub peer_config: Option<PathBuf>,
    #[arg(long = CLIENT_ACCESS_OPTION, conflicts_with_all = [BOOTSTRAP_FIELD, REMOTE_BOOTSTRAP_FIELD, REMOTE_RECOVERY_FIELD, INSTALL_BUNDLE_FIELD, INSPECT_INSTALL_FIELD])]
    pub client_access: Option<PathBuf>,
    #[arg(long = SSH_PROFILES_OPTION, conflicts_with_all = [BOOTSTRAP_FIELD, REMOTE_BOOTSTRAP_FIELD, REMOTE_RECOVERY_FIELD, INSTALL_BUNDLE_FIELD, INSPECT_INSTALL_FIELD])]
    pub ssh_profiles: Option<PathBuf>,
    #[arg(long = BROKER_EXECUTABLE_OPTION, conflicts_with_all = [BOOTSTRAP_FIELD, REMOTE_BOOTSTRAP_FIELD, REMOTE_RECOVERY_FIELD, INSTALL_BUNDLE_FIELD, INSPECT_INSTALL_FIELD])]
    pub broker_executable: Option<PathBuf>,
    #[arg(long = FILE_HELPER_OPTION, conflicts_with_all = [BOOTSTRAP_FIELD, REMOTE_BOOTSTRAP_FIELD, REMOTE_RECOVERY_FIELD, INSTALL_BUNDLE_FIELD, INSPECT_INSTALL_FIELD])]
    pub file_helper: Option<PathBuf>,
    #[arg(long = BOOTSTRAP_OPTION, requires = HOST_EXECUTABLE_FIELD, conflicts_with_all = [READY_FIELD, PEER_CONFIG_FIELD, HOST_SOCKET_FIELD])]
    pub bootstrap: bool,
    #[arg(long = HOST_EXECUTABLE_OPTION, requires = BOOTSTRAP_FIELD)]
    pub host_executable: Option<PathBuf>,
    #[arg(long = READY_OPTION, conflicts_with = BOOTSTRAP_FIELD)]
    pub ready_stdio: bool,
    #[arg(long = INSTALL_BUNDLE_OPTION, requires_all = [INSTALL_ROOT_FIELD, REQUEST_CAPNP_FIELD], conflicts_with_all = [INSPECT_INSTALL_FIELD, REMOTE_BOOTSTRAP_FIELD, BOOTSTRAP_FIELD, READY_FIELD, STATE_FIELD, HOST_HOME_FIELD, PEER_CONFIG_FIELD, HOST_SOCKET_FIELD])]
    pub install_bundle: bool,
    #[arg(long = INSPECT_INSTALL_OPTION, requires = INSTALL_ROOT_FIELD, conflicts_with_all = [REMOTE_BOOTSTRAP_FIELD, BOOTSTRAP_FIELD, READY_FIELD, REQUEST_CAPNP_FIELD, STATE_FIELD, HOST_HOME_FIELD, PEER_CONFIG_FIELD, HOST_SOCKET_FIELD])]
    pub inspect_install: bool,
    #[arg(long = REMOTE_BOOTSTRAP_OPTION, requires_all = [INSTALL_ROOT_FIELD, REQUEST_CAPNP_FIELD], conflicts_with_all = [BOOTSTRAP_FIELD, READY_FIELD, PEER_CONFIG_FIELD, HOST_SOCKET_FIELD, HOST_EXECUTABLE_FIELD])]
    pub remote_bootstrap: bool,
    #[arg(long = REMOTE_RECOVERY_OPTION, requires_all = [INSTALL_ROOT_FIELD, REQUEST_CAPNP_FIELD], conflicts_with_all = [BOOTSTRAP_FIELD, READY_FIELD, PEER_CONFIG_FIELD, HOST_SOCKET_FIELD, HOST_EXECUTABLE_FIELD])]
    pub recover_remote_bootstrap: bool,
    #[arg(long = INSTALL_ROOT_OPTION, requires = REMOTE_ACTION_GROUP)]
    pub install_root: Option<PathBuf>,
    #[arg(long = REQUEST_CAPNP_OPTION, requires = REMOTE_ACTION_GROUP)]
    pub request_capnp: Option<String>,
}
