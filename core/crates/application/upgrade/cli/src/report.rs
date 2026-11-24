use anyhow::Result;
use serde::Serialize;
use yolo_build_image::BuildImage;

#[derive(Debug, Serialize)]
pub(crate) struct CheckReport {
    pub(crate) status: &'static str,
    pub(crate) candidate: BuildImage,
    pub(crate) checks: Vec<CheckResult>,
}

#[derive(Debug, Serialize)]
pub(crate) struct CheckResult {
    pub(crate) name: String,
    pub(crate) passed: bool,
    pub(crate) detail: String,
}

impl CheckReport {
    pub(crate) fn new(candidate: BuildImage) -> Self {
        Self {
            status: crate::constants::REFUSED,
            candidate,
            checks: Vec::new(),
        }
    }
    pub(crate) fn push(
        &mut self,
        name: impl Into<String>,
        passed: bool,
        detail: impl Into<String>,
    ) {
        self.checks.push(CheckResult {
            name: name.into(),
            passed,
            detail: detail.into(),
        });
        self.status = if self.checks.iter().all(|check| check.passed) {
            crate::constants::READY
        } else {
            crate::constants::REFUSED
        };
    }
    pub(crate) fn ready(&self) -> bool {
        self.status == crate::constants::READY
    }

    pub(crate) fn to_json(&self) -> Result<String> {
        Ok(serde_json::to_string_pretty(self)?)
    }

    pub(crate) fn print(&self) {
        println!("upgrade {}", self.status);
        for check in &self.checks {
            println!(
                "{}: {} ({})",
                if check.passed { "ok" } else { "fail" },
                check.name,
                check.detail
            );
        }
    }
}
