use crate::github_release::{Channel, TARGET_TRIPLE, archive_asset, digest_for, is_newer, latest};
use serde_json::{Value, json};

const API: &str = "https://example.test/api/v3";
const REPOSITORY: &str = "custom/project";
const FULL_PAGE: usize = 100;
const STABLE: &str = "v0.2.0";
const NIGHTLY: &str = "nightly-20261103-deadbee";

fn stable() -> Value {
    json!({"tag_name": STABLE, "published_at": "2026-11-02T03:00:00Z"})
}

fn nightly() -> Value {
    json!({"tag_name": NIGHTLY, "prerelease": true, "published_at": "2026-11-03T03:00:00Z"})
}

#[test]
fn select_each_channel_beyond_a_full_page_in_the_configured_repository() {
    for (channel, first, mut target, tag) in [
        (Channel::Stable, nightly(), stable(), STABLE),
        (Channel::Nightly, stable(), nightly(), NIGHTLY),
    ] {
        let asset = format!("yolo-0.2.0-{TARGET_TRIPLE}.tar.gz");
        target["assets"] =
            json!([{"name": asset, "browser_download_url": "https://example.test/bin"}]);
        let mut draft = target.clone();
        draft["draft"] = json!(true);
        draft["published_at"] = json!("2026-11-04T03:00:00Z");
        let mut pages = [json!(vec![first; FULL_PAGE]), json!([draft, target])].into_iter();
        let mut requests = Vec::new();
        let release = latest(API, REPOSITORY, channel, |url| {
            requests.push(url.to_owned());
            Ok(pages.next().unwrap().to_string())
        })
        .unwrap();
        assert_eq!(release.tag_name, tag);
        assert_eq!(archive_asset(&release).unwrap().name, asset);
        assert_eq!(
            requests,
            [1, 2].map(|page| format!(
                "{API}/repos/{REPOSITORY}/releases?per_page={FULL_PAGE}&page={page}"
            ))
        );
        let digest = "a".repeat(64);
        assert_eq!(
            digest_for(&format!("{digest}  {asset}\n"), &asset).unwrap(),
            digest
        );
        assert!(digest_for("bad  other", &asset).is_err());
    }
}

#[test]
fn a_later_page_can_contain_the_last_published_release() {
    let mut later = stable();
    later["tag_name"] = json!("v0.3.0");
    later["published_at"] = json!("2026-11-05T03:00:00Z");
    let mut pages = [json!(vec![stable(); FULL_PAGE]), json!([later])].into_iter();
    let release = latest(API, REPOSITORY, Channel::Stable, |_| {
        Ok(pages.next().unwrap().to_string())
    })
    .unwrap();
    assert_eq!(release.tag_name, "v0.3.0");
}

#[test]
fn an_incomplete_listing_keeps_its_page_error_and_empty_listings_report_the_channel() {
    let mut first = Some(json!(vec![stable(); FULL_PAGE]).to_string());
    let error = latest(API, REPOSITORY, Channel::Stable, |_| {
        first
            .take()
            .ok_or_else(|| anyhow::anyhow!("service unavailable"))
    })
    .unwrap_err();
    assert!(format!("{error:#}").contains("page 2: service unavailable"));
    let invalid = latest(API, REPOSITORY, Channel::Stable, |_| Ok("invalid".into())).unwrap_err();
    assert!(
        invalid
            .to_string()
            .contains("invalid GitHub releases response on page 1")
    );
    let absent = latest(API, REPOSITORY, Channel::Nightly, |_| {
        Ok(json!([stable()]).to_string())
    })
    .unwrap_err();
    assert!(absent.to_string().contains("no Nightly YoloShell release"));
}

#[test]
fn compares_the_release_tag_with_the_version_embedded_by_the_release_jobs() {
    let release = latest(API, REPOSITORY, Channel::Nightly, |_| {
        Ok(json!([nightly()]).to_string())
    })
    .unwrap();
    let version = release.version();
    assert_eq!(version, "0.0.0-nightly.20261103.deadbee");
    assert!(!is_newer(&version, &version));
    assert!(!is_newer(&version, "0.0.0-nightly.20261104.abcdef1"));
    assert!(is_newer(&version, "0.0.0-nightly.20261102.abcdef1"));
    assert!(is_newer(&version, "0.0.0-nightly.20261103.abcdef1"));
    let release = latest(API, REPOSITORY, Channel::Stable, |_| {
        Ok(json!([stable()]).to_string())
    })
    .unwrap();
    assert_eq!(release.version(), "0.2.0");
    assert!(!is_newer(&release.version(), "0.10.0"));
}
