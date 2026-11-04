mod constants;
mod creation;
mod fixture;
mod negative;
mod persistent_namespace;
mod snapshot;

use anyhow::{Context, Result, bail, ensure};
use constants::*;
use std::path::PathBuf;

fn main() -> Result<()> {
    let mut arguments = std::env::args_os().skip(1);
    let stage = arguments.next().context(MISSING_STAGE)?;
    let root = PathBuf::from(arguments.next().context(USAGE)?);
    ensure!(arguments.next().is_none(), USAGE);
    match stage.to_str() {
        Some(SEED_STAGE) => fixture::seed(&root),
        Some(REOPEN_STAGE) => fixture::reopen(&root),
        Some(REJECT_COPY_STAGE) => negative::exercise(&root),
        _ => bail!(INVALID_STAGE),
    }
}
