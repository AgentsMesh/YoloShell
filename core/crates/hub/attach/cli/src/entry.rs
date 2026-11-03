use crate::{Arguments, run};
use clap::Parser;
use std::ffi::OsString;

/// Runs the terminal bridge role. Arguments include its display name as argv[0].
pub fn run_from(argv: impl IntoIterator<Item = OsString>) -> anyhow::Result<()> {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async move {
            run(Arguments::try_parse_from(argv)?).await?;
            Ok(())
        })
}
