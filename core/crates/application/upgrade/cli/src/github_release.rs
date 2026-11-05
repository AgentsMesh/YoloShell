use anyhow::{Context, Result, ensure};
use clap::ValueEnum;
use serde::Deserialize;

pub(crate) use crate::constants::TARGET_TRIPLE;

pub(crate) const DEFAULT_REPOSITORY: &str = "AgentsMesh/YoloShell";
pub(crate) const CHECKSUMS_ASSET: &str = "SHA256SUMS";

#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum)]
pub(crate) enum Channel {
    Stable,
    Nightly,
}

#[derive(Clone, Debug, Deserialize)]
pub(crate) struct Asset {
    pub(crate) name: String,
    pub(crate) browser_download_url: String,
}

#[derive(Clone, Debug, Deserialize)]
pub(crate) struct Release {
    pub(crate) tag_name: String,
    #[serde(default)]
    pub(crate) name: Option<String>,
    #[serde(default)]
    pub(crate) draft: bool,
    #[serde(default)]
    pub(crate) prerelease: bool,
    #[serde(default)]
    pub(crate) published_at: Option<String>,
    #[serde(default)]
    pub(crate) assets: Vec<Asset>,
}

pub(crate) fn parse(text: &str) -> Result<Vec<Release>> {
    serde_json::from_str(text).context("GitHub releases response is invalid")
}

pub(crate) fn latest(releases: &[Release], channel: Channel) -> Result<&Release> {
    let mut matching: Vec<&Release> = releases
        .iter()
        .filter(|release| {
            !release.draft
                && match channel {
                    Channel::Stable => !release.prerelease && release.tag_name.starts_with('v'),
                    Channel::Nightly => {
                        release.prerelease && release.tag_name.starts_with("nightly")
                    }
                }
        })
        .collect();
    matching.sort_by(|left, right| right.published_at.cmp(&left.published_at));
    matching
        .into_iter()
        .next()
        .with_context(|| format!("no {channel:?} YoloShell release is available"))
}

pub(crate) fn archive_asset<'a>(release: &'a Release) -> Result<&'a Asset> {
    let suffix = format!("-{TARGET_TRIPLE}.tar.gz");
    let matches: Vec<&Asset> = release
        .assets
        .iter()
        .filter(|asset| asset.name.ends_with(&suffix))
        .collect();
    ensure!(
        matches.len() == 1,
        "expected exactly one {suffix} release asset, found {}",
        matches.len()
    );
    Ok(matches[0])
}

pub(crate) fn checksums_asset(release: &Release) -> Result<&Asset> {
    release
        .assets
        .iter()
        .find(|asset| asset.name == CHECKSUMS_ASSET)
        .context("release has no SHA256SUMS asset")
}

pub(crate) fn digest_for(sums: &str, name: &str) -> Result<String> {
    let digest = sums.lines().find_map(|line| {
        let mut fields = line.split_whitespace();
        let digest = fields.next()?;
        let file = fields.next()?.trim_start_matches('*');
        (file == name && digest.len() == 64 && digest.chars().all(|c| c.is_ascii_hexdigit()))
            .then(|| digest.to_ascii_lowercase())
    });
    digest.with_context(|| format!("SHA256SUMS has no digest for {name}"))
}
