use crate::constants::*;
use crate::environment::Context;
use std::process::{Command, Output};

pub enum ExportFormat {
    Lcov,
    Json,
}

pub fn merge_profiles(context: &Context) -> Result<(), String> {
    let status = Command::new(&context.llvm_profdata)
        .arg(MERGE_COMMAND)
        .arg(SPARSE_ARGUMENT)
        .args(&context.raw_profiles)
        .arg(OUTPUT_ARGUMENT)
        .arg(&context.profile_data)
        .status()
        .map_err(|error| format!("cannot execute llvm-profdata: {error}"))?;
    if status.success() {
        Ok(())
    } else {
        Err(format!("llvm-profdata failed with {status}"))
    }
}

pub fn export(context: &Context, format: ExportFormat) -> Result<String, String> {
    let format_argument = match format {
        ExportFormat::Lcov => LCOV_FORMAT_ARGUMENT,
        ExportFormat::Json => JSON_FORMAT_ARGUMENT,
    };
    let mut command = Command::new(&context.llvm_cov);
    command
        .arg(EXPORT_COMMAND)
        .arg(format_argument)
        .arg(PROFILE_ARGUMENT)
        .arg(&context.profile_data)
        .arg(EXTERNAL_IGNORE_ARGUMENT)
        .arg(TEMPORARY_IGNORE_ARGUMENT)
        .arg(format!(
            "{PATH_EQUIVALENCE_PREFIX}{}",
            context.execution_root.display()
        ));
    add_instrumented_objects(&mut command, &context.instrumented_objects)?;
    let output = command
        .output()
        .map_err(|error| format!("cannot execute llvm-cov: {error}"))?;
    successful_output(output)
}

pub fn function_report(context: &Context, sources: &[String]) -> Result<String, String> {
    let output = function_report_command(context, sources)?
        .output()
        .map_err(|error| format!("cannot execute llvm-cov report: {error}"))?;
    successful_output(output)
}

pub(crate) fn function_report_command(
    context: &Context,
    sources: &[String],
) -> Result<Command, String> {
    if sources.is_empty() {
        return Err("cannot request an empty function report".to_owned());
    }
    let mut command = Command::new(&context.llvm_cov);
    command
        .arg(REPORT_COMMAND)
        .arg(PROFILE_ARGUMENT)
        .arg(&context.profile_data)
        .arg(SHOW_FUNCTIONS_ARGUMENT)
        .arg(SHOW_BRANCH_SUMMARY_ARGUMENT)
        .arg(SHOW_REGION_SUMMARY_ARGUMENT)
        .arg(format!(
            "{PATH_EQUIVALENCE_PREFIX}{}",
            context.execution_root.display()
        ));
    add_instrumented_objects(&mut command, &context.instrumented_objects)?;
    command.arg(SOURCES_ARGUMENT);
    for source in sources {
        command.arg(context.execution_root.join(source));
    }
    Ok(command)
}

pub(crate) fn add_instrumented_objects(
    command: &mut Command,
    objects: &[std::path::PathBuf],
) -> Result<(), String> {
    let Some((primary, additional)) = objects.split_first() else {
        return Err("coverage requires at least one instrumented object".to_owned());
    };
    command.arg(primary);
    for object in additional {
        command.arg(OBJECT_ARGUMENT).arg(object);
    }
    Ok(())
}

fn successful_output(output: Output) -> Result<String, String> {
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("llvm-cov failed with {}: {stderr}", output.status));
    }
    String::from_utf8(output.stdout)
        .map_err(|error| format!("llvm-cov output is not UTF-8: {error}"))
}
