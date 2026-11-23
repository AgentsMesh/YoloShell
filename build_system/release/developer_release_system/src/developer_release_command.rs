use crate::developer_release_arguments::*;
use crate::developer_release_constants::*;
use crate::developer_release_process::{run, run_with_file};
use anyhow::{Context, Result, ensure};
use serde::Deserialize;
use std::sync::Arc;
use yolo_release_contracts::{CODESIGN_EXECUTABLE, DITTO_EXECUTABLE};
use yolo_release_filesystem::DirectoryHandle;

pub trait DeveloperReleaseCommandRunner {
    fn extract(&self, workspace: &DirectoryHandle, archive: Arc<std::fs::File>) -> Result<()>;
    fn sign(&self, workspace: &DirectoryHandle) -> Result<()>;
    fn remove_signature(&self, workspace: &DirectoryHandle, path: &str) -> Result<()>;
    fn verify_signature(&self, workspace: &DirectoryHandle, extracted: bool) -> Result<()>;
    fn create_archive(
        &self,
        workspace: &DirectoryHandle,
        source: &str,
        destination: &str,
    ) -> Result<()>;
    fn notarize(
        &self,
        workspace: &DirectoryHandle,
        profile: &str,
        archive: Arc<std::fs::File>,
    ) -> Result<String>;
    fn staple(&self, workspace: &DirectoryHandle, extracted: bool) -> Result<()>;
    fn validate_ticket(&self, workspace: &DirectoryHandle, extracted: bool) -> Result<()>;
    fn assess(&self, workspace: &DirectoryHandle, extracted: bool) -> Result<()>;
}

pub struct SystemDeveloperReleaseCommandRunner;

impl DeveloperReleaseCommandRunner for SystemDeveloperReleaseCommandRunner {
    fn extract(&self, workspace: &DirectoryHandle, archive: Arc<std::fs::File>) -> Result<()> {
        run_with_file(
            workspace,
            DITTO_EXECUTABLE,
            extract_arguments(),
            "extract release candidate",
            archive,
        )?;
        Ok(())
    }

    fn sign(&self, workspace: &DirectoryHandle) -> Result<()> {
        for path in signing_paths(true) {
            run(
                workspace,
                CODESIGN_EXECUTABLE,
                sign_arguments(&path),
                "sign Yolo executable",
            )?;
        }
        let app = app_path(true);
        run(
            workspace,
            CODESIGN_EXECUTABLE,
            sign_arguments(&app),
            "sign Yolo application",
        )?;
        Ok(())
    }

    fn remove_signature(&self, workspace: &DirectoryHandle, path: &str) -> Result<()> {
        run(
            workspace,
            CODESIGN_EXECUTABLE,
            remove_signature_arguments(path),
            "normalize Yolo executable signature",
        )?;
        Ok(())
    }

    fn verify_signature(&self, workspace: &DirectoryHandle, extracted: bool) -> Result<()> {
        for path in signing_paths(extracted) {
            run(
                workspace,
                CODESIGN_EXECUTABLE,
                verify_arguments(&path, false),
                "verify Yolo executable signature",
            )?;
        }
        let app = app_path(extracted);
        run(
            workspace,
            CODESIGN_EXECUTABLE,
            verify_arguments(&app, true),
            "verify Yolo application signature",
        )?;
        Ok(())
    }

    fn create_archive(
        &self,
        workspace: &DirectoryHandle,
        source: &str,
        destination: &str,
    ) -> Result<()> {
        run(
            workspace,
            DITTO_EXECUTABLE,
            archive_arguments(source, destination),
            "archive signed Yolo application",
        )?;
        Ok(())
    }

    fn notarize(
        &self,
        workspace: &DirectoryHandle,
        profile: &str,
        archive: Arc<std::fs::File>,
    ) -> Result<String> {
        validate_profile(profile)?;
        let output = run_with_file(
            workspace,
            XCRUN_EXECUTABLE,
            notary_arguments(profile),
            "notarize Yolo application",
            archive,
        )?;
        parse_notary_response(&output.stdout)
    }

    fn staple(&self, workspace: &DirectoryHandle, extracted: bool) -> Result<()> {
        let arguments = stapler_arguments(STAPLER_STAPLE_SUBCOMMAND, &app_path(extracted));
        run(
            workspace,
            XCRUN_EXECUTABLE,
            arguments,
            "staple Yolo notarization ticket",
        )?;
        Ok(())
    }

    fn validate_ticket(&self, workspace: &DirectoryHandle, extracted: bool) -> Result<()> {
        let arguments = stapler_arguments(STAPLER_VALIDATE_SUBCOMMAND, &app_path(extracted));
        run(
            workspace,
            XCRUN_EXECUTABLE,
            arguments,
            "validate Yolo notarization ticket",
        )?;
        Ok(())
    }

    fn assess(&self, workspace: &DirectoryHandle, extracted: bool) -> Result<()> {
        run(
            workspace,
            SPCTL_EXECUTABLE,
            assessment_arguments(&app_path(extracted)),
            "assess Yolo with Gatekeeper",
        )?;
        Ok(())
    }
}

#[derive(Deserialize)]
struct NotaryResponse {
    id: String,
    status: String,
}

pub fn validate_profile(profile: &str) -> Result<()> {
    let mut bytes = profile.bytes();
    ensure!(
        !profile.is_empty()
            && profile.len() <= MAX_NOTARY_PROFILE_BYTES
            && profile.trim() == profile
            && bytes
                .next()
                .is_some_and(|byte| byte.is_ascii_alphanumeric())
            && bytes.all(valid_profile_byte),
        NOTARY_PROFILE_ERROR
    );
    Ok(())
}

pub fn parse_notary_response(bytes: &[u8]) -> Result<String> {
    let response: NotaryResponse = serde_json::from_slice(bytes).context(NOTARY_RESPONSE_ERROR)?;
    ensure!(
        response.status == NOTARY_ACCEPTED_STATUS && !response.id.trim().is_empty(),
        NOTARY_RESPONSE_ERROR
    );
    Ok(response.id)
}

fn valid_profile_byte(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.')
}
