#![allow(dead_code)]
use crate::constants::*;
use std::path::PathBuf;

pub fn binary(variable: &str) -> PathBuf {
    resolve(
        PathBuf::from(std::env::var_os(WORKSPACE_ENV).unwrap())
            .join(std::env::var_os(variable).unwrap()),
    )
}

pub fn settings() -> serde_json::Value {
    serde_json::from_slice(&std::fs::read(binary(FIXTURE_ENV)).unwrap()).unwrap()
}

pub fn tool(field: &str) -> PathBuf {
    resolve(PathBuf::from(settings()[field].as_str().unwrap()))
}

fn resolve(path: PathBuf) -> PathBuf {
    runfiles::Runfiles::create()
        .unwrap()
        .rlocation(&path)
        .unwrap_or_else(|| panic!("runfile is unavailable: {}", path.display()))
        .canonicalize()
        .unwrap()
}
