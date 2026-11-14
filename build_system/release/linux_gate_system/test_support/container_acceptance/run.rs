use super::{constants::*, fixture::FixtureTool, probe, receipt::Receipt};
use crate::container::{
    Owner,
    engine::{Engine, Selector, SystemEngine},
    model::Contract,
};
use anyhow::{Context, Result, ensure};
use std::fs::{self, File};
use std::io::Read;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use std::time::Instant;
use yolo_release_contracts::*;
use yolo_release_gate_system::{
    GateSnapshot, LINUX_ARM64_PROFILE, container_mounts, linux_gate_command,
};
use yolo_release_process_execution::ProcessCommand;
use yolo_release_tool_custody::{DockerClient, DockerClientTool};

pub fn run(path: &Path) -> Result<()> {
    let mut receipt = Receipt::create(path)?;
    let outcome = exercise(&mut receipt);
    if let Err(error) = &outcome {
        receipt.failure = Some(format!("{error:#}"));
        receipt.save()?;
    }
    outcome.with_context(|| format!("container owner acceptance receipt: {}", path.display()))
}

fn exercise(receipt: &mut Receipt) -> Result<()> {
    let docker = DockerClient::resolve()?;
    let snapshot = GateSnapshot::fixture()?;
    let tool = FixtureTool::create()?;
    let profile = LINUX_ARM64_PROFILE;
    let execution = snapshot.execution()?;
    let output = container_mounts::output_path(profile, &execution);
    fs::create_dir(&output)?;
    fs::set_permissions(&output, fs::Permissions::from_mode(PRIVATE_DIRECTORY_MODE))?;
    let user = format!("{}:{}", unsafe { libc::getuid() }, unsafe {
        libc::getgid()
    });
    let arguments = linux_gate_command::arguments(profile, &execution, tool.path(), &user)?;
    let contract = Contract::new(
        profile,
        &execution,
        tool.path(),
        CONTAINER_BUILD_ROLE,
        &arguments,
    )?;
    let engine = SystemEngine {
        snapshot: &snapshot,
        docker: &docker,
    };
    receipt.image = profile.image.into();
    receipt.engine_identity = engine.identity()?;
    receipt.container_name = contract.command.name.clone();
    receipt.snapshot_root = snapshot.root_path();
    receipt.tool_root = tool.root_path();
    receipt.command = arguments.clone();
    receipt.save()?;
    probe::image_is_cached(&snapshot, &docker, profile.image)?;
    let command = ProcessCommand::new(
        docker.path(),
        arguments,
        snapshot.source(),
        snapshot.home(),
        snapshot.temporary(),
        CLI_DESCRIPTION,
    )
    .retain_directories(docker.resource_directory())
    .retain_directories(Some(tool.directory()))
    .with_timeout(CLI_TIMEOUT)
    .with_output_limit(CONTAINER_CONTROL_OUTPUT_BYTES);
    snapshot.verify_integrity()?;
    tool.verify()?;
    let owner = Owner::prepare(&snapshot, &engine, contract, Some(tool.directory()))?;
    receipt.advance(OWNER_PREPARED)?;
    let launched = Instant::now();
    let outcome = command.execute();
    let error = match outcome {
        Err(error) => error,
        Ok(output) => anyhow::bail!("Docker CLI exited before forced timeout: {output:?}"),
    };
    let expected = format!("{CLI_DESCRIPTION} timed out");
    receipt.cli_error = Some(format!("{error:#}"));
    receipt.save()?;
    ensure!(
        format!("{error:#}") == expected,
        "CLI did not settle with only the deliberate timeout: {error:#}"
    );
    let current = engine
        .find(Selector::Name(&receipt.container_name))?
        .context("expected live container after settled Docker CLI timeout")?;
    Contract::new(
        profile,
        &execution,
        tool.path(),
        CONTAINER_BUILD_ROLE,
        &receipt.command,
    )?
    .verify(&current)?;
    receipt.container_id = Some(current.id.clone());
    receipt.save()?;
    let mut ready = Vec::new();
    File::open(output.join(READY_FILE))?
        .take(READY_MAX_BYTES)
        .read_to_end(&mut ready)?;
    ensure!(
        ready == READY_BYTES,
        "fixed build script did not reach fixture wait"
    );
    probe::is_running(&snapshot, &docker, &current.id)?;
    ensure!(
        engine.identity()? == receipt.engine_identity,
        ENGINE_CHANGED
    );
    receipt.advance(CLI_SETTLED)?;
    within_acceptance_budget(launched)?;
    let finished = owner
        .finish(Err(error))
        .err()
        .context("original CLI timeout must remain visible")?;
    ensure!(
        format!("{finished:#}") == expected,
        "engine cleanup failed: {finished:#}"
    );
    ensure!(
        engine.find(Selector::Id(&current.id))?.is_none(),
        "exact container ID survived Owner.finish"
    );
    ensure!(
        engine
            .find(Selector::Name(&receipt.container_name))?
            .is_none(),
        "owned name survived Owner.finish"
    );
    ensure!(
        engine.identity()? == receipt.engine_identity,
        ENGINE_CHANGED
    );
    within_acceptance_budget(launched)?;
    receipt.advance(ENGINE_ABSENT)?;
    snapshot.verify_integrity()?;
    tool.verify()?;
    drop(command);
    drop(engine);
    snapshot.close()?;
    tool.close()?;
    ensure!(
        !receipt.snapshot_root.exists() && !receipt.tool_root.exists(),
        "settled fixture resources were retained"
    );
    receipt.advance(RELEASED)
}

fn within_acceptance_budget(launched: Instant) -> Result<()> {
    let elapsed = launched.elapsed();
    ensure!(
        elapsed < ACCEPTANCE_BUDGET,
        "container owner acceptance exceeded {:?} from Docker launch ({elapsed:?}); \
         forced removal cannot be distinguished from the fixture's natural exit",
        ACCEPTANCE_BUDGET
    );
    Ok(())
}
