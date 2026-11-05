use anyhow::Result;
use yolo_local_lifecycle::{HostStatus, HubStatus, StatusReport, StopReport};

pub(crate) fn print_status(report: &StatusReport, json: bool) -> Result<()> {
    if json {
        println!("{}", serde_json::to_string_pretty(report)?);
    } else {
        status_lines(report)
            .iter()
            .for_each(|line| println!("{line}"));
    }
    Ok(())
}

pub(crate) fn print_version(report: &StatusReport, json: bool) -> Result<()> {
    if json {
        println!("{}", serde_json::to_string_pretty(report)?);
    } else {
        version_lines(report)
            .iter()
            .for_each(|line| println!("{line}"));
    }
    Ok(())
}

pub(crate) fn print_stop(report: &StopReport, json: bool) -> Result<()> {
    if json {
        println!("{}", serde_json::to_string_pretty(report)?);
    } else {
        print_stop_lines(report);
    }
    Ok(())
}

pub(crate) fn print_stop_lines(report: &StopReport) {
    for service in &report.services {
        match (service.stopped, service.pid) {
            (true, Some(pid)) => println!("stopped {} (pid {pid})", service.service),
            _ => println!("{} was not running", service.service),
        }
    }
}

pub(crate) fn status_lines(report: &StatusReport) -> Vec<String> {
    let mut lines = version_lines(report);
    lines.push(format!(
        "hub        {}{}{}",
        hub_state(&report.hub),
        image_suffix(report.hub.image.as_ref()),
        protocol_suffix(&report.hub)
    ));
    lines.push(format!(
        "host       {}{}",
        host_state(&report.host),
        image_suffix(report.host.image.as_ref())
    ));
    lines.push(format!(
        "file       {}",
        report
            .file_helper
            .as_ref()
            .map_or_else(|| "unavailable".into(), |image| short(image.sha256()))
    ));
    lines.push(format!(
        "state      {}",
        if report.state_contract {
            "ready"
        } else {
            "missing or incompatible"
        }
    ));
    lines.push(format!("images     {}", image_state(report)));
    if report.restart_needed {
        lines.push("restart    needed; run `yolo restart`".into());
    } else if !report.hub.running || !report.host.running {
        lines.push("start      run `yolo start`".into());
    } else {
        lines.push("restart    not needed".into());
    }
    lines
}

pub(crate) fn version_lines(report: &StatusReport) -> Vec<String> {
    let mut lines = vec![format!(
        "installed  yolo {}  image {}",
        report.client_version,
        short(report.client_image.sha256())
    )];
    let running = if report.hub.running || report.host.running {
        let hub = report
            .hub
            .image
            .as_ref()
            .map_or_else(|| "unknown".into(), |image| short(image.sha256()));
        let host = report
            .host
            .image
            .as_ref()
            .map_or_else(|| "unknown".into(), |image| short(image.sha256()));
        format!("hub {hub}  host {host}")
    } else {
        "none".into()
    };
    lines.push(format!("running    {running}"));
    lines
}

fn image_state(report: &StatusReport) -> &'static str {
    if !report.hub.running && !report.host.running {
        "not running"
    } else if report.live_images_consistent {
        "consistent"
    } else {
        "mixed or unavailable"
    }
}

fn hub_state(hub: &HubStatus) -> String {
    if !hub.running {
        return "stopped".into();
    }
    hub.pid
        .map_or_else(|| "running".into(), |pid| format!("running  pid {pid}"))
}

fn host_state(host: &HostStatus) -> String {
    if !host.running {
        return "stopped".into();
    }
    host.pid
        .map_or_else(|| "running".into(), |pid| format!("running  pid {pid}"))
}

fn image_suffix(image: Option<&yolo_build_image::BuildImage>) -> String {
    image.map_or_else(
        || "  image unavailable".into(),
        |image| format!("  image {}", short(image.sha256())),
    )
}

fn protocol_suffix(hub: &HubStatus) -> String {
    hub.protocol.map_or_else(
        || "  protocol unavailable".into(),
        |protocol| format!("  protocol {protocol}/{}", hub.expected_protocol),
    )
}

fn short(sha256: &str) -> String {
    sha256.chars().take(12).collect()
}
