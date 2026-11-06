use crate::constants::*;
use crate::install::Plan;
use crate::status::Row;
use serde_json::{Value, json};
use yolo_hook_installation::{HookInstallSpec, Installation, IntegrationStatus};

pub(crate) fn label(status: IntegrationStatus) -> &'static str {
    match status {
        IntegrationStatus::Missing => STATUS_NOT_INSTALLED,
        IntegrationStatus::Configured => STATUS_INSTALLED,
        IntegrationStatus::Restricted => STATUS_DISABLED,
    }
}

fn trust_label(required: bool) -> &'static str {
    if required {
        TRUST_REQUIRED
    } else {
        TRUST_NOT_REQUIRED
    }
}

pub(crate) fn status_line(row: &Row) -> String {
    format!(
        "{}\t{}\t{}\t{}\t{}",
        row.spec.provider,
        row.spec.display_name,
        label(row.preview.status),
        trust_label(row.spec.requires_provider_trust),
        row.preview.path.display()
    )
}

pub(crate) fn status_json(row: &Row) -> Value {
    json!({
        "provider": row.spec.provider,
        "agent": row.spec.display_name,
        "path": &row.preview.path,
        "status": &row.preview.status,
        "revision": &row.preview.revision,
        "requires_provider_trust": row.spec.requires_provider_trust,
    })
}

pub(crate) fn preview_json(plan: &Plan) -> Value {
    json!({
        "provider": plan.spec.provider,
        "agent": plan.spec.display_name,
        "path": &plan.preview.path,
        "status": &plan.preview.status,
        "revision": &plan.preview.revision,
        "requires_provider_trust": plan.spec.requires_provider_trust,
    })
}

pub(crate) fn install_json(plan: &Plan, installation: &Installation) -> Value {
    json!({
        "provider": plan.spec.provider,
        "agent": plan.spec.display_name,
        "path": &installation.preview.path,
        "status": &installation.preview.status,
        "changed": installation.changed,
        "backup": &installation.backup,
        "requires_provider_trust": plan.spec.requires_provider_trust,
    })
}

pub(crate) fn preview_lines(plan: &Plan) -> Vec<String> {
    let mut lines = vec![
        plan.spec.display_name.to_string(),
        format!("{FILE_PREFIX}{}", plan.preview.path.display()),
        format!("{STATUS_PREFIX}{}", label(plan.preview.status)),
        format!(
            "{EVENTS_PREFIX}{}{EVENTS_SUFFIX}",
            plan.spec.events.len()
        ),
        format!("{NOTE_PREFIX}{MANAGED_NOTE}"),
    ];
    if plan.spec.requires_provider_trust {
        lines.push(format!("{TRUST_PREFIX}{TRUST_NOTE}"));
    }
    lines
}

pub(crate) fn installed_lines(spec: &HookInstallSpec, installation: &Installation) -> Vec<String> {
    let mut lines = vec![format!(
        "{}{INSTALLED_SUFFIX}{}",
        spec.display_name,
        installation.preview.path.display()
    )];
    if let Some(backup) = &installation.backup {
        lines.push(format!("{BACKUP_PREFIX}{}", backup.display()));
    }
    lines
}

pub(crate) fn confirmation(names: &str) -> String {
    format!("{CONFIRM_PREFIX} {names}{CONFIRM_SUFFIX}")
}
