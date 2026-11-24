use super::{SYSTEMCTL_TIMEOUT, ServiceRole, ServiceUnit, service_config::property};
use anyhow::{Context, Result, ensure};

pub(super) async fn systemctl(user: bool, args: &[&str]) -> Result<String> {
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

pub(super) async fn systemctl_action(user: bool, action: &str, unit: &str) -> Result<()> {
    let _ = systemctl(user, &[action, "--", unit]).await?;
    Ok(())
}

pub(super) fn matching_units(
    listing: &str,
    hub_pid: Option<u32>,
    host_pid: Option<u32>,
    user: bool,
) -> Vec<ServiceUnit> {
    listing
        .split("\n\n")
        .filter_map(|unit| {
            let name = property(unit, "Id=")?;
            let pid = property(unit, "MainPID=")?.parse().ok()?;
            let role = if hub_pid == Some(pid) {
                Some(ServiceRole::Hub)
            } else if host_pid == Some(pid) {
                Some(ServiceRole::Host)
            } else {
                None
            }?;
            name.ends_with(".service").then(|| ServiceUnit {
                name: name.into(),
                pid,
                user,
                role,
            })
        })
        .collect()
}

pub async fn reload_managers(units: &[ServiceUnit]) -> Result<()> {
    let mut managers = units.iter().map(|unit| unit.user).collect::<Vec<_>>();
    managers.sort();
    managers.dedup();
    for user in managers {
        let _ = systemctl(user, &["daemon-reload"]).await?;
    }
    Ok(())
}
