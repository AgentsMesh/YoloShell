use crate::constants::*;
use clap::Parser;
use std::path::PathBuf;
use yolo_hub_identity::HubId;

#[derive(Parser)]
#[command(name = PROGRAM, version)]
pub struct Arguments {
    #[arg(long = SOCKET_OPTION)]
    pub socket: PathBuf,
    #[arg(long = HUB_OPTION)]
    pub hub: HubId,
}
