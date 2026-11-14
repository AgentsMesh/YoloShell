use super::*;

#[cfg(test)]
#[path = "reaper/pty_child_reaper.rs"]
pub(crate) mod pty_child_reaper;
#[path = "lifecycle/pty_exit_observer.rs"]
pub(crate) mod pty_exit_observer;
#[path = "reaper/pty_fault_reaper.rs"]
pub(crate) mod pty_fault_reaper;
#[path = "force/pty_force_termination.rs"]
pub(crate) mod pty_force_termination;
#[path = "lifecycle/pty_input_writer.rs"]
pub(crate) mod pty_input_writer;
#[path = "reaper/pty_missing_eof.rs"]
pub(crate) mod pty_missing_eof;
#[path = "output/pty_output_progress.rs"]
pub(crate) mod pty_output_progress;
#[path = "output/pty_output_terminal.rs"]
pub(crate) mod pty_output_terminal;
#[path = "owner/pty_owner.rs"]
pub(crate) mod pty_owner;
#[path = "owner/pty_owner_cleanup.rs"]
pub(crate) mod pty_owner_cleanup;
#[path = "owner/pty_owner_command_channel.rs"]
pub(crate) mod pty_owner_command_channel;
#[path = "owner/pty_owner_handle.rs"]
pub(crate) mod pty_owner_handle;
#[path = "owner/pty_owner_termination.rs"]
pub(crate) mod pty_owner_termination;
#[path = "ownership/pty_ownership.rs"]
pub(crate) mod pty_ownership;
#[path = "process/pty_process_custody.rs"]
pub(crate) mod pty_process_custody;
#[path = "reader/pty_reader_activation.rs"]
pub(crate) mod pty_reader_activation;
#[path = "alias/pty_runtime_alias_closure_task.rs"]
pub(crate) mod pty_runtime_alias_closure_task;
#[path = "alias/pty_runtime_alias_custody.rs"]
pub(crate) mod pty_runtime_alias_custody;

include!("base/pty.rs");
include!("base/pty_spawn.rs");
include!("reader/pty_reader_supervisor.rs");
include!("base/pty_tasks.rs");
include!("process/pty_processes.rs");
