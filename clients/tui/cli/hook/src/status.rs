use crate::{constants::*, render};
use anyhow::Result;
use std::io::{self, Write};
use yolo_agent_providers::builtin_install_specs;
use yolo_hook_installation::{HookInstallSpec, Preview, inspect, resolve_home};
use yolo_paths::validation::validate_absolute;

pub(crate) struct Row {
    pub(crate) spec: &'static HookInstallSpec,
    pub(crate) preview: Preview,
}

pub(crate) fn run(json: bool) -> Result<()> {
    let rows = rows()?;
    let mut out = io::stdout().lock();
    if json {
        let documents: Vec<_> = rows.iter().map(render::status_json).collect();
        serde_json::to_writer_pretty(&mut out, &documents)?;
        writeln!(out)?;
    } else {
        writeln!(out, "{STATUS_HEADER}")?;
        for row in &rows {
            writeln!(out, "{}", render::status_line(row))?;
        }
    }
    Ok(())
}

fn rows() -> Result<Vec<Row>> {
    builtin_install_specs()
        .into_iter()
        .map(|spec| {
            let home = resolve_home(spec, None, |name| std::env::var_os(name))?;
            validate_absolute(&home)?;
            Ok(Row {
                spec,
                preview: inspect(spec, &home)?,
            })
        })
        .collect()
}
