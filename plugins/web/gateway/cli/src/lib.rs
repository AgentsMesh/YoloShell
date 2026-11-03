//! Single-image Web gateway entrypoint. Never implicitly enrolls a client key.
mod constants;
mod options;
mod private;

use anyhow::Result;
use clap::Parser;
use std::{ffi::OsString, sync::Arc, time::Duration};
use yolo_hub_client_transport::HubPin;
use yolo_web_gateway::{AccessToken, Gateway, GatewayConfig, ScopedHub, WebOrigin};

pub use constants::COMMAND;
use constants::COMMAND_INDEX;

pub fn selects(arguments: &[OsString]) -> bool {
    arguments
        .get(COMMAND_INDEX)
        .is_some_and(|value| value == COMMAND)
}

pub fn run_from(arguments: impl IntoIterator<Item = OsString>) -> Result<()> {
    let arguments = options::Arguments::try_parse_from(arguments)?;
    let options::Command::Web(options) = arguments.command;
    let profile: options::Profile = yolo_hub_jsonrpc_protocol::decode(&private::read(
        &options.profile,
        yolo_web_gateway::MAX_PROFILE_BYTES,
    )?)
    .map_err(|_| anyhow::anyhow!("invalid scoped web Hub profile"))?;
    let document = private::read(&options.client_key, yolo_web_gateway::MAX_PRIVATE_KEY_BYTES)?;
    let key = russh::keys::decode_secret_key(std::str::from_utf8(&document)?, None)
        .map_err(|_| anyhow::anyhow!("invalid web client private key"))?;
    let token = private::read(&options.token_file, yolo_web_gateway::MAX_AUTH_BYTES as u64)?;
    let token = AccessToken::from_hex(std::str::from_utf8(&token)?.trim())?;
    let backend = ScopedHub::new(
        profile.socket,
        HubPin {
            hub_id: profile.hub_id,
            public_key: profile.hub_public_key,
        },
        profile.client,
        Arc::new(key),
    )?;
    let config = GatewayConfig::new(
        options.listen,
        options
            .origin
            .into_iter()
            .map(WebOrigin::parse)
            .collect::<Result<Vec<_>>>()?,
        token,
        Duration::from_secs(options.session_seconds),
        options.max_connections,
    )?;
    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?
        .block_on(async move {
            let gateway = Gateway::bind(config, backend).await?;
            println!(
                "Yolo Web gateway listening on ws://{}{}",
                gateway.local_addr()?,
                yolo_web_gateway::WEB_PATH
            );
            gateway
                .serve_until(async {
                    let _ = tokio::signal::ctrl_c().await;
                })
                .await
        })
}
