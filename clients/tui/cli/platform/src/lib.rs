mod constants;
mod platform;
mod platform_events;
mod platform_label;
mod platform_snapshot;

use clap::ArgGroup;

pub use platform::{PlatformCommand, run};

fn connection_group() -> ArgGroup {
    ArgGroup::new(constants::CONNECTION_GROUP)
        .args([constants::SOCKET_ARGUMENT, constants::ENDPOINT_ARGUMENT])
        .requires(constants::HUB_ARGUMENT)
}
