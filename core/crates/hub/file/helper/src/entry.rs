use clap::Parser;
use std::ffi::OsString;

#[derive(Parser)]
#[command(name = crate::constants::PROGRAM, version)]
struct Arguments {}

/// Runs the file helper role. Arguments include its display name as argv[0].
/// The process dispatcher must suppress runtime errors on the owner channel.
pub fn run_from(argv: impl IntoIterator<Item = OsString>) -> anyhow::Result<()> {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async move {
            Arguments::try_parse_from(argv)?;
            crate::run().await
        })
}
