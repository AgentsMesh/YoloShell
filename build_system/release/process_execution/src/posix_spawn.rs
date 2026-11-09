use crate::posix_spawn_actions::{FileActions, SpawnAttributes};
use crate::posix_spawn_descriptors::IsolatedSpawnDescriptors;
use crate::posix_spawn_strings::{argument_vector, environment_vector};
use crate::process::ProcessCommand;
use crate::process_descriptor::{set_close_on_exec, set_nonblocking};
use crate::process_spawn_lock::acquire as acquire_spawn_lock;
use anyhow::{Context, Result, anyhow, ensure};
use std::ffi::CString;
use std::fs::File;
use std::os::fd::{AsRawFd, FromRawFd};
use std::os::unix::ffi::OsStrExt;
use yolo_process::ProcessDiscoveryCapability;

#[path = "posix_spawn_child.rs"]
mod child;
pub(crate) use child::SpawnedProcess;

const SUSPENDED_ROLLBACK_CONTEXT: &str = "suspended process rollback failed";
const POSIX_SPAWN_CONTEXT: &str = "posix_spawn release command";
const PIPE_CREATE_CONTEXT: &str = "create process pipe";
const PIPE_ENDPOINT_COUNT: usize = 2;
const PIPE_READER_INDEX: usize = 0;
const PIPE_WRITER_INDEX: usize = 1;

pub(crate) fn spawn(specification: &ProcessCommand) -> Result<SpawnedProcess> {
    let mut resources = crate::process_resources::ProcessResources::acquire(specification)?;
    let spawn_lock = acquire_spawn_lock()?;
    let capability = ProcessDiscoveryCapability::generate()?;
    let inheritance = capability.begin_writer_inheritance()?;
    let (stdout_read, stdout_write) = output_pipe()?;
    let (stderr_read, stderr_write) = output_pipe()?;
    let input_pipe = specification.input.as_ref().map(|_| pipe()).transpose()?;
    let sources = IsolatedSpawnDescriptors::new(
        specification,
        &stdout_write,
        &stderr_write,
        input_pipe.as_ref().map(|(reader, _)| reader),
        Some(inheritance.descriptor()),
    )?;
    let mut actions = FileActions::new()?;
    actions.fchdir(specification.workspace.raw_fd())?;
    actions.dup(sources.standard_output.as_raw_fd(), libc::STDOUT_FILENO)?;
    actions.dup(sources.standard_error.as_raw_fd(), libc::STDERR_FILENO)?;
    if let Some(input) = &sources.standard_input {
        actions.dup(input.as_raw_fd(), libc::STDIN_FILENO)?;
    } else {
        actions.open_null_stdin()?;
    }
    for binding in &sources.inherited {
        actions.dup(binding.source.as_raw_fd(), binding.child_descriptor)?;
    }
    let mut attributes = SpawnAttributes::new()?;
    attributes.configure()?;
    let argv = argument_vector(specification)?;
    let environment = environment_vector(specification, &capability)?;
    let executable = CString::new(specification.executable.as_os_str().as_bytes())?;
    let mut pid = libc::pid_t::default();
    let status = unsafe {
        libc::posix_spawn(
            &mut pid,
            executable.as_ptr(),
            &actions.value,
            &attributes.value,
            argv.pointers.as_ptr(),
            environment.pointers.as_ptr(),
        )
    };
    if status != libc::c_int::default() {
        return Err(std::io::Error::from_raw_os_error(status)).context(POSIX_SPAWN_CONTEXT);
    }
    resources.mark_spawned();
    drop(actions);
    drop(sources);
    drop(stdout_write);
    drop(stderr_write);
    let stdin = if let Some((input_read, input_write)) = input_pipe {
        drop(input_read);
        Some(input_write)
    } else {
        None
    };
    let mut child = SpawnedProcess::new(pid, stdout_read, stderr_read, stdin);
    child.resources = resources;
    if let Err(capture) = child.install_custody(capability, inheritance) {
        return match child.rollback_failed_capture() {
            Ok(()) => Err(capture),
            Err(rollback) => Err(anyhow!(
                "{capture:#}; {SUSPENDED_ROLLBACK_CONTEXT}: {rollback:#}"
            )),
        };
    }
    drop(spawn_lock);
    Ok(child)
}

fn pipe() -> Result<(File, File)> {
    let mut descriptors = [libc::c_int::default(); PIPE_ENDPOINT_COUNT];
    ensure!(
        unsafe { libc::pipe(descriptors.as_mut_ptr()) } == libc::c_int::default(),
        PIPE_CREATE_CONTEXT
    );
    let read = unsafe { File::from_raw_fd(descriptors[PIPE_READER_INDEX]) };
    let write = unsafe { File::from_raw_fd(descriptors[PIPE_WRITER_INDEX]) };
    set_close_on_exec(read.as_raw_fd())?;
    set_close_on_exec(write.as_raw_fd())?;
    Ok((read, write))
}

fn output_pipe() -> Result<(File, File)> {
    let (read, write) = pipe()?;
    set_nonblocking(read.as_raw_fd())?;
    Ok((read, write))
}
