use anyhow::{bail, Context, Result};
use std::path::Path;
use std::process::Command;

use crate::constants::{
    GIT, GIT_BRANCH, GIT_CLONE, GIT_DEPTH, GIT_DEPTH_VALUE, GIT_DIRECTORY, GIT_HEAD, GIT_NO_TAGS,
    GIT_OPTIONS_END, GIT_PROMPT_DISABLED, GIT_REV_PARSE, GIT_TERMINAL_PROMPT,
};

pub(crate) fn clone_registry(repository: &str, destination: &Path) -> Result<()> {
    clone(repository, None, destination)
}

pub(crate) fn clone_source(repository: &str, tag: &str, destination: &Path) -> Result<()> {
    clone(repository, Some(tag), destination)
}

fn clone(repository: &str, tag: Option<&str>, destination: &Path) -> Result<()> {
    let mut command = Command::new(GIT);
    command
        .args([GIT_CLONE, GIT_DEPTH, GIT_DEPTH_VALUE, GIT_NO_TAGS])
        .env(GIT_TERMINAL_PROMPT, GIT_PROMPT_DISABLED);
    if let Some(tag) = tag {
        command.args([GIT_BRANCH, tag]);
    }
    command.args([GIT_OPTIONS_END, repository]).arg(destination);
    run(&mut command).with_context(|| format!("clone {repository}"))
}

pub(crate) fn head(repository: &Path) -> Result<String> {
    let mut command = Command::new(GIT);
    command
        .args([GIT_DIRECTORY])
        .arg(repository)
        .args([GIT_REV_PARSE, GIT_HEAD]);
    let output = command
        .output()
        .context("git is required to install plugins")?;
    if !output.status.success() {
        bail!(
            "cannot read cloned plugin commit: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }
    let head = String::from_utf8(output.stdout)?.trim().to_owned();
    if head.len() != 40 || !head.chars().all(|character| character.is_ascii_hexdigit()) {
        bail!("git returned an invalid plugin commit");
    }
    Ok(head)
}

pub(crate) fn copy_plugin(source: &Path, destination: &Path) -> Result<()> {
    std::fs::create_dir_all(destination)
        .with_context(|| format!("create {}", destination.display()))?;
    copy_tree(source, destination)?;
    Ok(())
}

fn copy_tree(source: &Path, destination: &Path) -> Result<()> {
    let mut copied = false;
    for entry in std::fs::read_dir(source)
        .with_context(|| format!("read plugin source {}", source.display()))?
    {
        let entry = entry?;
        let name = entry.file_name();
        if name == ".git" {
            continue;
        }
        let path = entry.path();
        let metadata = std::fs::symlink_metadata(&path)?;
        let target = destination.join(&name);
        if metadata.file_type().is_symlink() {
            bail!("plugin source must not contain symlinks: {}", path.display());
        }
        if metadata.is_dir() {
            std::fs::create_dir(&target)
                .with_context(|| format!("create {}", target.display()))?;
            copy_tree(&path, &target)?;
            copied = true;
        } else if metadata.is_file() {
            std::fs::copy(&path, &target)
                .with_context(|| format!("copy {}", path.display()))?;
            copied = true;
        } else {
            bail!("plugin source contains a non-file entry: {}", path.display());
        }
    }
    if !copied {
        bail!("plugin source is empty: {}", source.display());
    }
    Ok(())
}

fn run(command: &mut Command) -> Result<()> {
    let output = command
        .output()
        .context("git is required to install plugins")?;
    if !output.status.success() {
        bail!(
            "git failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }
    Ok(())
}
