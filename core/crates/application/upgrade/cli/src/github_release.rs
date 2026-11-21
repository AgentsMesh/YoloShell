use anyhow::{Context, Result, ensure};
use clap::ValueEnum;
use serde::Deserialize;

pub(crate) use crate::constants::TARGET_TRIPLE;

pub(crate) const DEFAULT_REPOSITORY: &str = "AgentsMesh/YoloShell";
pub(crate) const CHECKSUMS_ASSET: &str = "SHA256SUMS";
const RELEASES_PER_PAGE: usize = 100;
const NIGHTLY_VERSION_PREFIX: &str = "0.0.0-nightly.";

#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum)]
pub enum Channel {
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

impl Release {
    fn matches(&self, channel: Channel) -> bool {
        !self.draft
            && match channel {
                Channel::Stable => !self.prerelease && self.tag_name.starts_with('v'),
                Channel::Nightly => self.prerelease && self.tag_name.starts_with("nightly"),
            }
    }

    pub(crate) fn version(&self) -> String {
        if let Some((date, revision)) = self
            .tag_name
            .strip_prefix("nightly-")
            .and_then(|suffix| nightly_parts(suffix, '-'))
        {
            return format!("{NIGHTLY_VERSION_PREFIX}{date}.{revision}");
        }
        self.tag_name.trim_start_matches('v').to_owned()
    }
}

pub(crate) fn latest(
    api: &str,
    repository: &str,
    channel: Channel,
    mut get: impl FnMut(&str) -> Result<String>,
) -> Result<Release> {
    let api = api.trim_end_matches('/');
    let mut latest: Option<Release> = None;
    for page in 1usize.. {
        let url =
            format!("{api}/repos/{repository}/releases?per_page={RELEASES_PER_PAGE}&page={page}");
        let body = get(&url).with_context(|| format!("read GitHub releases page {page}"))?;
        let releases: Vec<Release> = serde_json::from_str(&body)
            .with_context(|| format!("invalid GitHub releases response on page {page}"))?;
        let last_page = releases.len() < RELEASES_PER_PAGE;
        // The list endpoint has no published_at sort parameter. A release published
        // from an older tag can be the newest in the selected channel on a later page.
        for release in releases {
            if release.matches(channel)
                && latest
                    .as_ref()
                    .is_none_or(|old| release.published_at > old.published_at)
            {
                latest = Some(release);
            }
        }
        if last_page {
            break;
        }
    }
    latest.with_context(|| format!("no {channel:?} YoloShell release is available"))
}

pub(crate) fn is_newer(candidate: &str, current: &str) -> bool {
    let nightly = |version: &str| {
        version
            .strip_prefix(NIGHTLY_VERSION_PREFIX)
            .and_then(|suffix| nightly_parts(suffix, '.'))
            .and_then(|(date, _)| date.parse::<u32>().ok())
    };
    match (nightly(candidate), nightly(current)) {
        (Some(candidate_date), Some(current_date)) if candidate_date != current_date => {
            candidate_date > current_date
        }
        _ => match (components(candidate), components(current)) {
            (Some(candidate), Some(current)) => candidate > current,
            _ => candidate != current,
        },
    }
}

fn nightly_parts(suffix: &str, separator: char) -> Option<(&str, &str)> {
    let (date, revision) = suffix.split_once(separator)?;
    (date.len() == 8
        && date.bytes().all(|byte| byte.is_ascii_digit())
        && !revision.is_empty()
        && revision.bytes().all(|byte| byte.is_ascii_hexdigit()))
    .then_some((date, revision))
}

fn components(version: &str) -> Option<[u64; 3]> {
    let mut parts = version.split('.');
    let mut values = [0u64; 3];
    for value in &mut values {
        *value = parts.next()?.parse().ok()?;
    }
    parts.next().is_none().then_some(values)
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
