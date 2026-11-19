use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use yolo_trusted_plugin_manifest::{TrustedPluginManifest, load};

use crate::LifecycleContext;

pub const TIMEOUT: Duration = Duration::from_secs(30);
const POLL: Duration = Duration::from_millis(20);

#[derive(Debug, thiserror::Error)]
pub enum LifecycleError {
    #[error("plugin manifest is invalid: {0}")]
    Manifest(#[from] yolo_trusted_plugin_manifest::ManifestError),
    #[error("plugin has no startup hook")]
    NoStartup,
    #[error("plugin action is not declared")]
    UnknownAction,
    #[error("plugin command `{program}` could not start: {source}")]
    Start {
        program: String,
        #[source]
        source: std::io::Error,
    },
    #[error("plugin lifecycle I/O failed: {0}")]
    Io(#[from] std::io::Error),
    #[error("plugin spawn gate is unavailable")]
    SpawnGate,
    #[error("plugin command timed out")]
    Timeout,
    #[error("plugin command failed")]
    Failed,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LifecycleOutcome {
    pub success: bool,
    pub log: std::path::PathBuf,
}

pub fn run_startup(context: &LifecycleContext) -> Result<LifecycleOutcome, LifecycleError> {
    let manifest = load(&context.root)?;
    run_startup_with(context, &manifest)
}

pub fn run_startup_with(
    context: &LifecycleContext,
    manifest: &TrustedPluginManifest,
) -> Result<LifecycleOutcome, LifecycleError> {
    let command = manifest.startup.as_ref().ok_or(LifecycleError::NoStartup)?;
    execute(manifest, &command.command, context)
}

pub fn run_action(
    context: &LifecycleContext,
    action_id: &str,
) -> Result<LifecycleOutcome, LifecycleError> {
    let manifest = load(&context.root)?;
    run_action_with(context, action_id, &manifest)
}

pub fn run_action_with(
    context: &LifecycleContext,
    action_id: &str,
    manifest: &TrustedPluginManifest,
) -> Result<LifecycleOutcome, LifecycleError> {
    let action = manifest
        .actions
        .iter()
        .find(|action| action.id == action_id)
        .ok_or(LifecycleError::UnknownAction)?;
    let mut context = context.clone();
    context.action_id = Some(action_id.into());
    execute(manifest, &action.command, &context)
}

fn execute(
    manifest: &TrustedPluginManifest,
    argv: &[String],
    context: &LifecycleContext,
) -> Result<LifecycleOutcome, LifecycleError> {
    crate::log::private_dir(&context.log)?;
    let log_path = context.log.join(format!(
        "{}-{}.log",
        context.action_id.as_deref().unwrap_or("startup"),
        now_ms()
    ));
    let log = crate::log::create(&log_path)?;
    let mut command = Command::new(&argv[0]);
    command
        .args(&argv[1..])
        .current_dir(&context.root)
        .env("YOLO_PLUGIN_ID", &context.plugin_id)
        .env("YOLO_PLUGIN_ROOT", &context.root)
        .env("YOLO_PLUGIN_DATA", &context.data)
        .env("YOLO_HUB_SOCKET", &context.hub_socket)
        .env("YOLO_HUB_SOCKET_KIND", "local_management")
        .env(
            "YOLO_PLUGIN_ACTION_ID",
            context.action_id.as_deref().unwrap_or("startup"),
        )
        .env("YOLO_PLUGIN_CONTEXT_JSON", context_json(manifest, context)?)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    if let Some(hub_id) = context.hub_id {
        command.env("YOLO_HUB_ID", hub_id.to_string());
    }
    #[cfg(unix)]
    unsafe {
        use std::os::unix::process::CommandExt;
        command.pre_exec(|| {
            if libc::setsid() == -1 {
                return Err(std::io::Error::last_os_error());
            }
            Ok(())
        });
    }
    let deadline = Instant::now() + TIMEOUT;
    let guard = yolo_process_spawn::waitable_child_spawn_guard_before(deadline)
        .map_err(|_| LifecycleError::SpawnGate)?;
    let mut child = guard.boundary().spawn_command(&mut command).map_err(|source| {
        LifecycleError::Start {
            program: argv[0].clone(),
            source,
        }
    })?;
    drop(guard);
    let mut readers = Vec::new();
    if let Some(pipe) = child.stdout.take() {
        readers.push(crate::log::spawn(pipe));
    }
    if let Some(pipe) = child.stderr.take() {
        readers.push(crate::log::spawn(pipe));
    }
    let status = loop {
        if let Some(status) = child.try_wait()? {
            break status;
        }
        if Instant::now() >= deadline {
            #[cfg(unix)]
            unsafe {
                libc::kill(-(child.id() as i32), libc::SIGKILL);
            }
            let _ = child.wait();
            crate::log::write(&log, readers, context);
            return Err(LifecycleError::Timeout);
        }
        std::thread::sleep(POLL);
    };
    crate::log::write(&log, readers, context);
    if !status.success() {
        return Err(LifecycleError::Failed);
    }
    Ok(LifecycleOutcome {
        success: true,
        log: log_path,
    })
}

fn context_json(
    manifest: &TrustedPluginManifest,
    context: &LifecycleContext,
) -> Result<String, std::io::Error> {
    serde_json::to_string(&serde_json::json!({
        "plugin_id": manifest.id,
        "action_id": context.action_id,
        "profile": "trusted",
    }))
    .map_err(std::io::Error::other)
}

fn now_ms() -> u128 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or_default()
}
