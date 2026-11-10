use crate::{command, constants::*};
use anyhow::{Result, ensure};
use std::path::Path;
use yolo_distribution_bundle::{Architecture, ArtifactRole, Platform, Target};
use yolo_remote::MultiplexedSshConnection;

pub(crate) async fn target(
    connection: &dyn MultiplexedSshConnection,
    target: Target,
) -> Result<()> {
    let platform = command::run(connection, PROBE_PROGRAM, vec![PLATFORM_ARGUMENT.into()]).await?;
    let architecture = command::run(
        connection,
        PROBE_PROGRAM,
        vec![ARCHITECTURE_ARGUMENT.into()],
    )
    .await?;
    let platform = std::str::from_utf8(&platform)?.trim_end_matches('\n');
    let architecture = std::str::from_utf8(&architecture)?.trim_end_matches('\n');
    ensure!(
        platform
            == match target.platform {
                Platform::Linux => PLATFORM_LINUX,
                Platform::MacOs => PLATFORM_MACOS,
            },
        TARGET_MISMATCH
    );
    ensure!(
        match target.architecture {
            Architecture::Aarch64 => ARCH_ARM,
            Architecture::X86_64 => ARCH_X86,
        }
        .contains(&architecture),
        TARGET_MISMATCH
    );
    Ok(())
}

pub(crate) async fn version(
    connection: &dyn MultiplexedSshConnection,
    program: &Path,
    role: ArtifactRole,
    version: &str,
) -> Result<()> {
    let program = program
        .to_str()
        .ok_or_else(|| anyhow::anyhow!(INVALID_PATH))?;
    let output = command::run(connection, program, vec![VERSION_ARGUMENT.into()]).await?;
    ensure!(
        output == format!("{} {version}\n", role.executable_name()).as_bytes(),
        VERSION_MISMATCH
    );
    Ok(())
}
