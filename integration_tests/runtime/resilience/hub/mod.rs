#[path = "agent_control.rs"]
mod agent_control;
#[path = "agent/agent_directory.rs"]
mod agent_directory;
#[path = "attach/attach_binary.rs"]
mod attach_binary;
#[path = "attach/attach_federation.rs"]
mod attach_federation;
#[path = "attach/attach_support.rs"]
mod attach_support;
#[path = "catalog/catalog_gate.rs"]
mod catalog_gate;
#[cfg(target_os = "macos")]
#[path = "ssh/cold_ssh_setup.rs"]
mod cold_ssh_setup;
#[path = "connector/connector.rs"]
mod connector;
mod conversation;
#[path = "creation/creation_cleanup.rs"]
mod creation_cleanup;
#[path = "creation/creation_support.rs"]
mod creation_support;
#[path = "catalog/catalog_federation.rs"]
mod catalog_federation;
#[path = "public/public_client_support.rs"]
mod public_client_support;
#[path = "federation/federation_agents.rs"]
mod federation_agents;
#[path = "federation/federation_creation_query.rs"]
mod federation_creation_query;
#[path = "federation/federation_flow.rs"]
mod federation_flow;
#[path = "federation/federation_hubs.rs"]
mod federation_hubs;
#[path = "federation/federation_observe_only.rs"]
mod federation_observe_only;
#[path = "federation/federation_support.rs"]
mod federation_support;
#[path = "federation/federation_tui.rs"]
mod federation_tui;
#[path = "fixture_agents.rs"]
mod fixture_agents;
#[path = "files/file_root_ui_constants.rs"]
mod file_root_ui_constants;
#[path = "files/file_root_ui_driver.rs"]
mod file_root_ui_driver;
#[path = "files/files_constants.rs"]
mod files_constants;
#[path = "files/files_flow.rs"]
mod files_flow;
#[path = "files/files_gate_support.rs"]
mod files_gate_support;
#[path = "files/files_revocation.rs"]
mod files_revocation;
#[path = "files/files_runtime.rs"]
mod files_runtime;
#[path = "files/files_support.rs"]
mod files_support;
#[path = "files/files_tui.rs"]
mod files_tui;
#[path = "startup/local_startup.rs"]
mod local_startup;
#[path = "startup/local_startup_support.rs"]
mod local_startup_support;
#[path = "management/management.rs"]
mod management;
#[path = "management/management_support.rs"]
mod management_support;
#[path = "management/management_tui.rs"]
mod management_tui;
#[path = "management/management_tui_driver.rs"]
mod management_tui_driver;
#[path = "multipath/multipath.rs"]
mod multipath;
#[path = "native/native_control.rs"]
mod native_control;
#[cfg(target_os = "macos")]
#[path = "native/native_management.rs"]
mod native_management;
#[cfg(target_os = "macos")]
#[path = "native/native_management_support.rs"]
mod native_management_support;
#[cfg(target_os = "macos")]
#[path = "native/native_workspace.rs"]
mod native_workspace;
#[cfg(target_os = "macos")]
#[path = "native/native_workspace_support.rs"]
mod native_workspace_support;
mod offline_inventory;
#[cfg(target_os = "macos")]
#[path = "public/public_client_tests.rs"]
mod public_client;
#[path = "relay/relay_crash.rs"]
mod relay_crash;
#[path = "remote/remote_bundle.rs"]
mod remote_bundle;
#[cfg(target_os = "macos")]
#[path = "ssh_profile_live.rs"]
mod ssh_profile_live;
mod support;
#[path = "tui_clipboard/tui_clipboard.rs"]
mod tui_clipboard;
mod tui_chat;
#[path = "tui_command/constants.rs"]
mod tui_command;
#[path = "tui_creation/tui_creation.rs"]
mod tui_creation;
#[path = "tui_creation/tui_creation_recovery.rs"]
mod tui_creation_recovery;
#[path = "tui_creation/tui_finalization.rs"]
mod tui_finalization;
#[cfg(target_os = "macos")]
#[path = "tui_local_startup/tui_local_startup.rs"]
mod tui_local_startup;
mod tui_tabs;
mod tui_ux;
#[path = "tui_queries/tui_queries.rs"]
mod tui_queries;
mod tui_resize_input;
#[path = "tui_driver/tui_input_latency.rs"]
mod tui_input_latency;
#[path = "tui_shell_first/tui_shell_first.rs"]
mod tui_shell_first;
#[path = "tui_workspaces/tui_workspaces.rs"]
mod tui_workspaces;
mod tui_writer_recovery;
mod writer_reconnect;
