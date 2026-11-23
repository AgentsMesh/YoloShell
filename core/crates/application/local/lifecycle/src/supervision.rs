use anyhow::{Context, Result, ensure};
use std::{path::PathBuf, time::Duration};
use yolo_hub_state::HubState;
use yolo_local_profile::LocalProfile;

const SYSTEMCTL_TIMEOUT: Duration = Duration::from_secs(2);

#[derive(Debug)]
pub struct ServiceUnit {
    pub name: String,
    pid: u32,
    user: bool,
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
    let (hub, host) = tokio::join!(crate::service_pid(&hub), crate::service_pid(&host));
    let pids: Vec<_> = [host, hub].into_iter().flatten().collect();
    if pids.is_empty() {
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
        units.extend(matching_units(&listing, &pids, user));
    }
    Ok(units)
}

impl ServiceUnit {
    /// Read the effective command from the manager after applying drop-ins.
    /// A launcher can take the pinned image path as its only argument.
    pub async fn image_path(&self) -> Result<PathBuf> {
        let effective = systemctl(
            self.user,
            &[
                "show",
                "--property=MainPID",
                "--property=NeedDaemonReload",
                "--property=ExecStart",
                "--",
                &self.name,
            ],
        )
        .await?;
        ensure!(
            property(&effective, "MainPID=").and_then(|pid| pid.parse().ok()) == Some(self.pid),
            "service owner changed during admission"
        );
        ensure!(
            property(&effective, "NeedDaemonReload=") == Some("no"),
            "service configuration needs systemd daemon-reload"
        );
        image_path(property(&effective, "ExecStart=").context("missing effective ExecStart")?)
    }
}

fn matching_units(listing: &str, pids: &[u32], user: bool) -> Vec<ServiceUnit> {
    listing
        .split("\n\n")
        .filter_map(|unit| {
            let name = property(unit, "Id=")?;
            let pid = property(unit, "MainPID=")?.parse().ok()?;
            (name.ends_with(".service") && pids.contains(&pid)).then(|| ServiceUnit {
                name: name.into(),
                pid,
                user,
            })
        })
        .collect()
}

fn image_path(command: &str) -> Result<PathBuf> {
    let command = command
        .strip_prefix("{ path=")
        .context("unknown ExecStart format")?;
    let (path, argv) = command
        .split_once(" ; argv[]=")
        .context("missing ExecStart arguments")?;
    let path = PathBuf::from(path);
    ensure!(path.is_absolute(), "service ExecStart must be absolute");
    if path.file_name().is_some_and(|name| name == "yolo") {
        return Ok(path);
    }
    let (argv, _) = argv
        .split_once(" ; ")
        .context("missing ExecStart argument list")?;
    let image = argv
        .strip_prefix(path.to_str().context("invalid service launcher path")?)
        .and_then(|args| args.strip_prefix(' '))
        .context("the service launcher must take the immutable yolo image as its only argument")?;
    let image = PathBuf::from(image);
    ensure!(
        image.is_absolute() && image.file_name().is_some_and(|name| name == "yolo"),
        "the service launcher must take the immutable yolo image as its only argument"
    );
    Ok(image)
}

fn property<'a>(text: &'a str, key: &str) -> Option<&'a str> {
    text.lines().find_map(|line| line.strip_prefix(key))
}

async fn systemctl(user: bool, args: &[&str]) -> Result<String> {
    let mut command = tokio::process::Command::new("/usr/bin/systemctl");
    command
        .arg(if user { "--user" } else { "--system" })
        .arg("--no-pager")
        .args(args)
        .kill_on_drop(true);
    if user {
        command.env(
            "XDG_RUNTIME_DIR",
            format!("/run/user/{}", unsafe { libc::geteuid() }),
        );
        command.env_remove("DBUS_SESSION_BUS_ADDRESS");
    }
    let output = tokio::time::timeout(SYSTEMCTL_TIMEOUT, command.output())
        .await
        .context("systemd service inspection timed out")??;
    ensure!(
        output.status.success(),
        "could not inspect the selected profile's systemd services"
    );
    Ok(String::from_utf8(output.stdout)?)
}
