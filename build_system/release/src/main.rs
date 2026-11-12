use anyhow::Result;
use clap::{Parser, Subcommand};
use std::path::PathBuf;
use yolo_release_evidence::{
    ACCEPTANCE_RECEIPT_ARGUMENT, ARCHIVE_ARGUMENT, AUDIT_ARCHIVE_OPERATION, AUDIT_BUNDLE_OPERATION,
    AUDIT_MANIFEST_OPERATION, CANDIDATE_DIRECTORY_ARGUMENT, CANDIDATE_OPERATION, CandidateRequest,
    DELIVER_OPERATION, DISTRIBUTE_OPERATION, DeliveryRequest, DeveloperReleaseRequest,
    MANIFEST_ARGUMENT, NOTARY_PROFILE_ARGUMENT, OUTPUT_DIRECTORY_ARGUMENT,
    RELEASE_EVIDENCE_CLI_NAME, VERIFY_ACCEPTANCE_OPERATION, VERIFY_OPERATION,
    VerifyAcceptanceRequest, VerifyRequest, WORKSPACE_ARGUMENT, audit_archive_path,
    audit_bundle_manifest_path, audit_runtime_manifest_path, create_candidate, deliver, distribute,
    verify, verify_acceptance,
};

#[derive(Debug, Parser)]
#[command(name = RELEASE_EVIDENCE_CLI_NAME)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    #[command(name = AUDIT_BUNDLE_OPERATION)]
    AuditBundle {
        #[arg(long = MANIFEST_ARGUMENT)]
        manifest: PathBuf,
    },
    #[command(name = AUDIT_ARCHIVE_OPERATION)]
    AuditArchive {
        #[arg(long = ARCHIVE_ARGUMENT)]
        archive: PathBuf,
    },
    #[command(name = AUDIT_MANIFEST_OPERATION)]
    AuditManifest {
        #[arg(long = MANIFEST_ARGUMENT)]
        manifest: PathBuf,
    },
    #[command(name = CANDIDATE_OPERATION)]
    Candidate {
        #[arg(long = WORKSPACE_ARGUMENT)]
        workspace: PathBuf,
        #[arg(long = OUTPUT_DIRECTORY_ARGUMENT)]
        output_directory: PathBuf,
    },
    #[command(name = DELIVER_OPERATION)]
    Deliver {
        #[arg(long = CANDIDATE_DIRECTORY_ARGUMENT)]
        candidate_directory: PathBuf,
        #[arg(long = OUTPUT_DIRECTORY_ARGUMENT)]
        output_directory: PathBuf,
    },
    #[command(name = DISTRIBUTE_OPERATION)]
    Distribute {
        #[arg(long = CANDIDATE_DIRECTORY_ARGUMENT)]
        candidate_directory: PathBuf,
        #[arg(long = OUTPUT_DIRECTORY_ARGUMENT)]
        output_directory: PathBuf,
        #[arg(long = NOTARY_PROFILE_ARGUMENT)]
        notary_profile: String,
        #[arg(long = ACCEPTANCE_RECEIPT_ARGUMENT)]
        acceptance_receipt: PathBuf,
    },
    #[command(name = VERIFY_OPERATION)]
    Verify {
        #[arg(long = OUTPUT_DIRECTORY_ARGUMENT)]
        output_directory: PathBuf,
    },
    #[command(name = VERIFY_ACCEPTANCE_OPERATION)]
    VerifyAcceptance {
        #[arg(long = CANDIDATE_DIRECTORY_ARGUMENT)]
        candidate_directory: PathBuf,
        #[arg(long = ACCEPTANCE_RECEIPT_ARGUMENT)]
        acceptance_receipt: PathBuf,
    },
}

fn main() -> Result<()> {
    match Cli::parse().command {
        Command::AuditArchive { archive } => audit_archive_path(&archive).map(|_| ()),
        Command::AuditManifest { manifest } => audit_runtime_manifest_path(&manifest),
        Command::AuditBundle { manifest } => audit_bundle_manifest_path(&manifest),
        Command::Candidate {
            workspace,
            output_directory,
        } => create_candidate(CandidateRequest {
            workspace: &workspace,
            output_directory: &output_directory,
        })
        .map(|_| ()),
        Command::Deliver {
            candidate_directory,
            output_directory,
        } => deliver(DeliveryRequest {
            candidate_directory: &candidate_directory,
            output_directory: &output_directory,
        })
        .map(|_| ()),
        Command::Distribute {
            candidate_directory,
            output_directory,
            notary_profile,
            acceptance_receipt,
        } => distribute(DeveloperReleaseRequest {
            candidate_directory: &candidate_directory,
            output_directory: &output_directory,
            notary_profile: &notary_profile,
            acceptance_receipt: Some(&acceptance_receipt),
        })
        .map(|_| ()),
        Command::Verify { output_directory } => verify(VerifyRequest {
            output_directory: &output_directory,
        })
        .map(|_| ()),
        Command::VerifyAcceptance {
            candidate_directory,
            acceptance_receipt,
        } => verify_acceptance(VerifyAcceptanceRequest {
            candidate_directory: &candidate_directory,
            acceptance_receipt: &acceptance_receipt,
        })
        .map(|_| ()),
    }
}
