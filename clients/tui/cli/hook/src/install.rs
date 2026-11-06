use crate::{constants::*, render};
use anyhow::{Context, Result, ensure};
use std::io::{self, IsTerminal, Write};
use std::path::{Path, PathBuf};
use yolo_agent_providers::builtin_install_spec;
use yolo_hook_installation::{
    HookInstallSpec, IntegrationStatus, Preview, install, inspect, resolve_home,
};
use yolo_paths::validation::validate_absolute;

pub(crate) struct Plan {
    pub(crate) spec: &'static HookInstallSpec,
    pub(crate) home: PathBuf,
    pub(crate) preview: Preview,
}

pub(crate) fn run(
    providers: &[String],
    config_home: Option<&Path>,
    yes: bool,
    dry_run: bool,
    json: bool,
) -> Result<()> {
    ensure!(
        config_home.is_none() || providers.len() == 1,
        CONFIG_HOME_SINGLE_PROVIDER
    );
    let plans = build_plans(providers, config_home)?;
    for plan in &plans {
        ensure!(
            plan.preview.status != IntegrationStatus::Restricted,
            "{}: {RESTRICTED}",
            plan.spec.display_name
        );
    }
    let pending: Vec<&Plan> = plans
        .iter()
        .filter(|plan| plan.preview.status == IntegrationStatus::Missing)
        .collect();
    if json {
        return run_json(&plans, &pending, yes, dry_run);
    }
    for plan in &plans {
        render::preview_lines(plan).iter().for_each(|line| println!("{line}"));
    }
    if pending.is_empty() {
        println!("{ALREADY_INSTALLED}");
        return Ok(());
    }
    if dry_run {
        println!("{DRY_RUN}");
        return Ok(());
    }
    if !yes {
        ensure!(io::stdin().is_terminal(), NON_INTERACTIVE);
        if !confirm(&pending)? {
            println!("{CANCELLED}");
            return Ok(());
        }
    }
    install_pending(&pending)
}

fn run_json(plans: &[Plan], pending: &[&Plan], yes: bool, dry_run: bool) -> Result<()> {
    let documents = if dry_run || pending.is_empty() {
        plans.iter().map(render::preview_json).collect()
    } else {
        ensure!(yes, JSON_REQUIRES_YES);
        let mut documents = Vec::with_capacity(plans.len());
        for plan in plans {
            let installation = install(plan.spec, &plan.home, &plan.preview.revision)?;
            documents.push(render::install_json(plan, &installation));
        }
        documents
    };
    let mut out = io::stdout().lock();
    serde_json::to_writer_pretty(&mut out, &documents)?;
    writeln!(out)?;
    Ok(())
}

fn build_plans(providers: &[String], config_home: Option<&Path>) -> Result<Vec<Plan>> {
    providers
        .iter()
        .map(|token| plan(token, config_home))
        .collect()
}

fn plan(token: &str, config_home: Option<&Path>) -> Result<Plan> {
    let spec = builtin_install_spec(token).with_context(|| unknown_provider(token))?;
    let home = resolve_home(spec, config_home.map(Path::to_path_buf), |name| {
        std::env::var_os(name)
    })?;
    validate_absolute(&home)?;
    Ok(Plan {
        spec,
        preview: inspect(spec, &home)?,
        home,
    })
}

fn unknown_provider(token: &str) -> String {
    let known: Vec<_> = yolo_agent_providers::builtin_install_specs()
        .iter()
        .map(|spec| spec.provider)
        .collect();
    format!(
        "{UNKNOWN_PROVIDER} `{token}`; {KNOWN_PROVIDERS}: {}",
        known.join(", ")
    )
}

fn confirm(pending: &[&Plan]) -> Result<bool> {
    let names: Vec<_> = pending.iter().map(|plan| plan.spec.display_name).collect();
    print!("{}", render::confirmation(&names.join(", ")));
    io::stdout().flush()?;
    let mut answer = String::new();
    io::stdin().read_line(&mut answer)?;
    Ok(matches!(
        answer.trim().to_ascii_lowercase().as_str(),
        "y" | "yes"
    ))
}

fn install_pending(pending: &[&Plan]) -> Result<()> {
    for plan in pending {
        let installation = install(plan.spec, &plan.home, &plan.preview.revision)?;
        render::installed_lines(plan.spec, &installation)
            .iter()
            .for_each(|line| println!("{line}"));
        if plan.spec.requires_provider_trust {
            println!("{}: {TRUST_NOTE}", plan.spec.display_name);
        }
    }
    Ok(())
}
