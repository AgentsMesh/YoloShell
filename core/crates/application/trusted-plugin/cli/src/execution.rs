use std::path::{Path, PathBuf};

use anyhow::Result;
use yolo_trusted_plugin_lifecycle::{
    LifecycleContext, run_action_with, run_startup_with,
};
use yolo_trusted_plugin_registry::{Registry, RegistryPaths, StartupStatus};

pub(crate) fn startup(paths: &RegistryPaths, json: bool) -> Result<()> {
    let registry = Registry::load(paths)?;
    let ids = registry
        .records()
        .filter(|record| {
            record.enabled
                && record
                    .platforms
                    .iter()
                    .any(|platform| platform.runs_on(yolo_plugin_api::Platform::current()))
        })
        .map(|record| record.id.clone())
        .collect::<Vec<_>>();
    let endpoint = (!ids.is_empty()).then(endpoint).transpose()?;
    let mut failed = false;
    for id in ids {
        match registry.revalidate(&id) {
            Ok((_, manifest)) if manifest.startup.is_none() => continue,
            Ok(_) => {}
            Err(error) => {
                failed = true;
                eprintln!("{id}: {error}");
                let _ = update_startup(
                    paths,
                    &id,
                    StartupStatus {
                        success: false,
                        finished_at_unix_ms: now_ms(),
                    },
                );
                continue;
            }
        }
        let result = endpoint
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("plugin endpoint is unavailable"))
            .and_then(|endpoint| execute(&registry, paths, &id, endpoint, None));
        if let Err(error) = result {
            failed = true;
            eprintln!("{id}: {error}");
            let _ = update_startup(
                paths,
                &id,
                StartupStatus {
                    success: false,
                    finished_at_unix_ms: now_ms(),
                },
            );
        }
    }
    if json {
        let updated = Registry::load(paths)?;
        println!(
            "{}",
            serde_json::to_string_pretty(&updated.records().collect::<Vec<_>>())?
        );
    }
    anyhow::ensure!(!failed, "one or more plugins failed to start");
    Ok(())
}

pub(crate) fn invoke_action(
    paths: &RegistryPaths,
    id: &str,
    action: &str,
    json: bool,
) -> Result<()> {
    let registry = Registry::load(paths)?;
    let result = execute(&registry, paths, id, &endpoint()?, Some(action))?;
    if json {
        println!("{}", serde_json::to_string_pretty(&result)?);
    }
    Ok(())
}

pub(crate) fn execute(
    registry: &Registry,
    paths: &RegistryPaths,
    id: &str,
    endpoint: &Endpoint,
    action: Option<&str>,
) -> Result<serde_json::Value> {
    let (record, manifest) = registry.revalidate(id)?;
    let record = record.clone();
    let context = LifecycleContext {
        plugin_id: id.into(),
        root: record.source,
        data: paths.data_dir(id),
        log: paths.log_dir(id),
        hub_socket: endpoint.socket.clone(),
        hub_id: endpoint.hub,
        action_id: action.map(str::to_owned),
    };
    private_dir(&context.data)?;
    let outcome = match action {
        Some(action) => run_action_with(&context, action, &manifest)?,
        None => run_startup_with(&context, &manifest)?,
    };
    if action.is_none() {
        update_startup(
            paths,
            id,
            StartupStatus {
                success: true,
                finished_at_unix_ms: now_ms(),
            },
        )?;
    }
    Ok(serde_json::json!({"id": id, "success": true, "log": outcome.log}))
}

fn update_startup(
    paths: &RegistryPaths,
    id: &str,
    status: StartupStatus,
) -> Result<(), yolo_trusted_plugin_registry::RegistryError> {
    let mut registry = Registry::load_locked(paths)?;
    registry.set_startup(id, status, paths)
}

pub(crate) struct Endpoint {
    pub(crate) socket: PathBuf,
    pub(crate) hub: Option<yolo_hub_identity::HubId>,
}

fn endpoint() -> Result<Endpoint> {
    if let Some(socket) = std::env::var_os("YOLO_HUB_SOCKET") {
        let raw = std::env::var("YOLO_HUB_ID")
            .map_err(|_| anyhow::anyhow!("YOLO_HUB_SOCKET requires YOLO_HUB_ID"))?;
        let hub = raw
            .parse()
            .map_err(|_| anyhow::anyhow!("YOLO_HUB_ID is invalid"))?;
        return Ok(Endpoint {
            socket: PathBuf::from(socket),
            hub: Some(hub),
        });
    }
    let profile = yolo_local_profile::current()?;
    let executable = std::env::current_exe()?;
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;
    let launch = yolo_hub_local_startup::Launch {
        state: profile.state,
        host_home: profile.host_home,
        host_executable: executable.clone(),
        hub_executable: executable,
    };
    let deadline = tokio::time::Instant::now() + yolo_hub_local_startup::STARTUP_TIMEOUT;
    let document = runtime.block_on(yolo_hub_local_startup::ensure_running(launch, deadline))?;
    Ok(Endpoint {
        socket: document.socket_path,
        hub: Some(document.entry_hub),
    })
}

fn private_dir(path: &Path) -> std::io::Result<()> {
    yolo_paths::directory::PrivateDirectory::ensure(path).map(|_| ()).map_err(std::io::Error::other)
}

fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|value| value.as_millis() as u64)
        .unwrap_or_default()
}
