//! Manual acceptance for engine-side ownership after bounded CLI custody settles.
use anyhow::{Result, ensure};
use std::path::Path;

mod constants;
#[path = "../../src/container/mod.rs"]
mod container;
mod fixture;
mod probe;
mod receipt;
mod run;

fn main() -> Result<()> {
    let arguments = std::env::args_os().skip(1).collect::<Vec<_>>();
    let [receipt] = arguments.as_slice() else {
        anyhow::bail!(constants::USAGE);
    };
    ensure!(
        cfg!(target_os = "macos"),
        "acceptance requires macOS custody"
    );
    run::run(Path::new(receipt))
}
