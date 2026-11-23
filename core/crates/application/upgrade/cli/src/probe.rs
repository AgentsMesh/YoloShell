use crate::{check::Options, constants::*, report::CheckReport, service::ServiceUnit};
use anyhow::Result;
use std::time::Duration;
use tokio::time::Instant;
use yolo_build_image::BuildImage;
use yolo_hub_client::Client;
use yolo_hub_rpc_protocol::{Command, HUB_PROTOCOL_VERSION, Reply};
use yolo_hub_state::validate_runtime_contract;
use yolo_local_profile::LocalProfile;

pub(crate) async fn probe(
    options: &Options,
    profile: &LocalProfile,
    candidate: BuildImage,
) -> Result<CheckReport> {
    let mut report = CheckReport::new(candidate.clone());
    report.push(
        CHECK_CANDIDATE,
        true,
        format!("{} bytes", candidate.size_bytes()),
    );
    let deadline = Instant::now() + Duration::from_secs(CHECK_DEADLINE_SECONDS);
    let client = Client::connect_local(
        options.socket.as_ref().unwrap(),
        options.hub.unwrap(),
        deadline,
    )
    .await;
    let mut client = match client {
        Ok(client) => client,
        Err(error) => {
            report.push(
                CHECK_PROTOCOL,
                false,
                format!("cannot contact running Hub: {error}"),
            );
            check_state_and_units(options, profile, &candidate, &mut report).await;
            return Ok(report);
        }
    };
    let hello = client.hello().clone();
    report.push(
        CHECK_PROTOCOL,
        hello.version == HUB_PROTOCOL_VERSION,
        format!(
            "running={}, expected={}",
            hello.version, HUB_PROTOCOL_VERSION
        ),
    );
    // Hello consumed the connect budget; the build probe owns a fresh one.
    match client
        .call(
            options.hub.unwrap(),
            Command::GetBuild {},
            Instant::now() + Duration::from_secs(CHECK_DEADLINE_SECONDS),
        )
        .await
    {
        Ok(Reply::Build { build }) => {
            report.push(
                CHECK_BUILD,
                build.all_match(&candidate),
                format!(
                    "host={}, hub={}, file_helper={}",
                    build.host.sha256(),
                    build.hub.sha256(),
                    build.file_helper.sha256()
                ),
            );
        }
        Ok(reply) => report.push(
            CHECK_BUILD,
            false,
            format!("unexpected Hub reply: {reply:?}"),
        ),
        Err(error) => report.push(
            CHECK_BUILD,
            false,
            format!("cannot read live service images: {error}"),
        ),
    }
    check_state_and_units(options, profile, &candidate, &mut report).await;
    Ok(report)
}

async fn check_state_and_units(
    options: &Options,
    profile: &LocalProfile,
    candidate: &BuildImage,
    report: &mut CheckReport,
) {
    let state = validate_runtime_contract(options.state.as_ref().unwrap());
    report.push(
        CHECK_STATE,
        state.is_ok(),
        state
            .err()
            .map_or_else(|| CONTRACT_MATCH.into(), |error| error.to_string()),
    );
    if options.service_units.is_empty() {
        check_supervised_units(profile, candidate, report).await;
    }
    for path in &options.service_units {
        let name = format!("{CHECK_SERVICE_PREFIX}{}", path.display());
        match ServiceUnit::read(path) {
            Ok(unit) => report.push(
                name,
                &unit.image == candidate,
                format!("{}", unit.executable.display()),
            ),
            Err(error) => report.push(name, false, error.to_string()),
        }
    }
}

async fn check_supervised_units(
    profile: &LocalProfile,
    candidate: &BuildImage,
    report: &mut CheckReport,
) {
    let units = match yolo_local_lifecycle::service_units(profile).await {
        Ok(units) => units,
        Err(error) => {
            report.push(CHECK_SERVICE_UNITS, false, error.to_string());
            return;
        }
    };
    if units.is_empty() {
        report.push(
            CHECK_SERVICE_UNITS,
            true,
            "profile uses independently launched services",
        );
    }
    for unit in units {
        let observed = async {
            let path = unit.image_path().await?;
            crate::service::verify_pin(&path, candidate)?;
            Ok::<_, anyhow::Error>(path)
        }
        .await;
        let name = format!("{CHECK_SERVICE_PREFIX}{}", unit.name);
        match observed {
            Ok(path) => report.push(name, true, path.display().to_string()),
            Err(error) => report.push(name, false, error.to_string()),
        }
    }
}
