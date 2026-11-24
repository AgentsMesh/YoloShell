mod manager;
mod service_config;

use anyhow::{Context, Result, ensure};
use std::{
    fs,
    path::{Path, PathBuf},
    time::Duration,
};
use yolo_hub_state::HubState;
use yolo_local_profile::LocalProfile;

const SYSTEMCTL_TIMEOUT: Duration = Duration::from_secs(2);

use manager::{matching_units, systemctl, systemctl_action};
use service_config::{command_paths, managed_service_file, property, replace_image};

pub use manager::reload_managers;

#[derive(Debug)]
pub struct ServiceUnit {
    pub name: String,
    pid: u32,
    user: bool,
    role: ServiceRole,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ServiceRole {
    Hub,
    Host,
}

/// Resolve the supervisor from the selected profile's private socket owners.
/// A Host wrapper may move its PID to a separate PTY custody cgroup, so use
/// systemd's MainPID instead of the process's current cgroup.
pub async fn service_units(profile: &LocalProfile) -> Result<Vec<ServiceUnit>> {
    if !cfg!(target_os = "linux") {
        return Ok(Vec::new());
    }
    let hub = HubState::socket_path_for(&profile.state)?;
    let host = yolo_paths::Paths::new(&profile.host_home).host_socket()?;
    let (hub_pid, host_pid) = tokio::join!(crate::service_pid(&hub), crate::service_pid(&host));
    if hub_pid.is_none() && host_pid.is_none() {
        return Ok(Vec::new());
    }
    let runtime = format!("/run/user/{}/systemd/private", unsafe { libc::geteuid() });
    let mut units = Vec::new();
    for (user, socket) in [(false, "/run/systemd/private"), (true, runtime.as_str())] {
        if !std::path::Path::new(socket).exists() {
            continue;
        }
        let listing = systemctl(
            user,
            &[
                "show",
                "--type=service",
                "--property=Id",
                "--property=MainPID",
                "--",
                "*.service",
            ],
        )
        .await?;
        units.extend(matching_units(&listing, hub_pid, host_pid, user));
    }
    Ok(units)
}

impl ServiceUnit {
    pub fn pid(&self) -> u32 {
        self.pid
    }

    pub fn role(&self) -> ServiceRole {
        self.role
    }
    /// Read the effective command from the manager after applying drop-ins.
    /// A launcher can take the pinned image path as its only argument.
    pub async fn image_path(&self) -> Result<PathBuf> {
        Ok(self.effective().await?.image)
    }

    pub async fn stop(&self) -> Result<()> {
        systemctl_action(self.user, "stop", &self.name).await
    }

    pub async fn start(&self) -> Result<()> {
        systemctl_action(self.user, "start", &self.name).await
    }

    pub async fn activate_image(&self, candidate: &Path) -> Result<()> {
        let effective = self.effective().await?;
        let old_image = effective.image.clone();
        let unmanaged = effective
            .files
            .iter()
            .filter(|path| !managed_service_file(path, &self.name))
            .filter(|path| {
                fs::read_to_string(path)
                    .map(|text| old_image.to_str().is_some_and(|image| text.contains(image)))
                    .unwrap_or(true)
            })
            .collect::<Vec<_>>();
        ensure!(
            unmanaged.is_empty(),
            "effective image is overridden by unmanaged service configuration: {}",
            unmanaged
                .iter()
                .map(|path| path.display().to_string())
                .collect::<Vec<_>>()
                .join(", ")
        );
        let mut paths = effective
            .files
            .into_iter()
            .filter(|path| managed_service_file(path, &self.name))
            .collect::<Vec<_>>();
        if effective.executable != old_image && effective.executable.is_file() {
            ensure!(
                service_config::managed_executable(&effective.executable),
                "service launcher is outside the managed executable roots: {}",
                effective.executable.display()
            );
            paths.push(effective.executable);
        }
        paths.sort();
        paths.dedup();
        for path in paths {
            replace_image(&path, &old_image, candidate)?;
        }
        Ok(())
    }

    async fn effective(&self) -> Result<EffectiveService> {
        let text = systemctl(
            self.user,
            &[
                "show",
                "--property=MainPID",
                "--property=NeedDaemonReload",
                "--property=ExecStart",
                "--property=FragmentPath",
                "--property=DropInPaths",
                "--",
                &self.name,
            ],
        )
        .await?;
        ensure!(
            property(&text, "MainPID=").and_then(|pid| pid.parse().ok()) == Some(self.pid),
            "service owner changed during admission"
        );
        ensure!(
            property(&text, "NeedDaemonReload=") == Some("no"),
            "service configuration needs systemd daemon-reload"
        );
        let command = property(&text, "ExecStart=").context("missing effective ExecStart")?;
        let (executable, image) = command_paths(command)?;
        let mut files = property(&text, "FragmentPath=")
            .into_iter()
            .chain(
                property(&text, "DropInPaths=")
                    .into_iter()
                    .flat_map(str::split_whitespace),
            )
            .filter(|path| !path.is_empty())
            .map(PathBuf::from)
            .collect::<Vec<_>>();
        files.retain(|path| path.is_file());
        Ok(EffectiveService {
            executable,
            image,
            files,
        })
    }
}

#[derive(Debug)]
struct EffectiveService {
    executable: PathBuf,
    image: PathBuf,
    files: Vec<PathBuf>,
}
