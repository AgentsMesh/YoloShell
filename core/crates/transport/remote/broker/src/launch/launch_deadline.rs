use anyhow::{Context, Result, ensure};
use std::fmt::{Display, Formatter};
use std::str::FromStr;
use std::sync::OnceLock;
use std::time::Duration;

const DEADLINE_VERSION: u16 = 1;
const FIELD_SEPARATOR: char = '.';
const DEADLINE_EXPIRED: &str = "broker launch deadline has expired";
const DEADLINE_BOOT_UNKNOWN: &str = "broker launch deadline boot identity is unavailable";
const DEADLINE_BOOT_MISMATCH: &str = "broker launch deadline belongs to another boot";
const DEADLINE_ENCODING_INVALID: &str = "broker launch deadline encoding is invalid";
const DEADLINE_NANOSECONDS_INVALID: &str = "broker launch deadline nanoseconds are invalid";
const DEADLINE_OVERFLOW: &str = "broker launch deadline overflow";
const DEADLINE_ANCHOR_SKEW: &str = "broker launch monotonic anchor skew is excessive";
const SHA256_HEX_BYTES: usize = 64;
const MONOTONIC_ANCHOR_SAMPLE_ATTEMPTS: usize = 8;
const MONOTONIC_ANCHOR_MAX_SKEW: Duration = Duration::from_millis(1);

static MONOTONIC_ANCHOR: OnceLock<MonotonicAnchor> = OnceLock::new();

#[derive(Clone, Copy)]
struct MonotonicAnchor {
    before_nanoseconds: u128,
    instant: tokio::time::Instant,
    after_nanoseconds: u128,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BrokerLaunchDeadline {
    boot_sha256: String,
    monotonic_nanoseconds: u128,
}

impl BrokerLaunchDeadline {
    pub fn capture(deadline: tokio::time::Instant) -> Result<Self> {
        ensure!(deadline > tokio::time::Instant::now(), DEADLINE_EXPIRED);
        let anchor = monotonic_anchor()?;
        let budget = deadline
            .checked_duration_since(anchor.instant)
            .context(DEADLINE_EXPIRED)?;
        let monotonic_nanoseconds = anchor
            .before_nanoseconds
            .checked_add(budget.as_nanos())
            .context(DEADLINE_OVERFLOW)?;
        Ok(Self {
            boot_sha256: current_boot_sha256()?,
            monotonic_nanoseconds,
        })
    }

    pub fn to_instant(&self) -> Result<tokio::time::Instant> {
        ensure!(
            self.boot_sha256 == current_boot_sha256()?,
            DEADLINE_BOOT_MISMATCH
        );
        let anchor = monotonic_anchor()?;
        let remaining = self
            .monotonic_nanoseconds
            .checked_sub(anchor.after_nanoseconds)
            .context(DEADLINE_EXPIRED)?;
        ensure!(remaining > u128::default(), DEADLINE_EXPIRED);
        let nanoseconds = u64::try_from(remaining).context(DEADLINE_NANOSECONDS_INVALID)?;
        let deadline = anchor
            .instant
            .checked_add(Duration::from_nanos(nanoseconds))
            .context(DEADLINE_OVERFLOW)?;
        ensure!(deadline > tokio::time::Instant::now(), DEADLINE_EXPIRED);
        Ok(deadline)
    }
}

impl Display for BrokerLaunchDeadline {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            formatter,
            "{}{FIELD_SEPARATOR}{}{FIELD_SEPARATOR}{}",
            DEADLINE_VERSION, self.boot_sha256, self.monotonic_nanoseconds
        )
    }
}

impl FromStr for BrokerLaunchDeadline {
    type Err = anyhow::Error;

    fn from_str(value: &str) -> Result<Self> {
        let mut fields = value.split(FIELD_SEPARATOR);
        let version = fields.next().context(DEADLINE_ENCODING_INVALID)?;
        let boot_sha256 = fields.next().context(DEADLINE_ENCODING_INVALID)?;
        let nanoseconds = fields.next().context(DEADLINE_ENCODING_INVALID)?;
        ensure!(fields.next().is_none(), DEADLINE_ENCODING_INVALID);
        ensure!(
            version == DEADLINE_VERSION.to_string(),
            DEADLINE_ENCODING_INVALID
        );
        ensure!(valid_sha256(boot_sha256), DEADLINE_ENCODING_INVALID);
        let monotonic_nanoseconds = nanoseconds.parse::<u128>()?;
        ensure!(
            nanoseconds == monotonic_nanoseconds.to_string(),
            DEADLINE_ENCODING_INVALID
        );
        Ok(Self {
            boot_sha256: boot_sha256.to_owned(),
            monotonic_nanoseconds,
        })
    }
}

fn current_boot_sha256() -> Result<String> {
    let boot = crate::process::boot_id();
    ensure!(
        crate::process::boot_identity_relation(&boot, &boot)
            == crate::process::BootIdentityRelation::Same,
        DEADLINE_BOOT_UNKNOWN
    );
    Ok(yolo_crypto::sha256_hex(boot.as_bytes()))
}

fn valid_sha256(value: &str) -> bool {
    value.len() == SHA256_HEX_BYTES
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || matches!(byte, b'a'..=b'f'))
}

fn monotonic_nanoseconds() -> Result<u128> {
    let mut value = libc::timespec {
        tv_sec: libc::time_t::default(),
        tv_nsec: libc::c_long::default(),
    };
    let status = unsafe { libc::clock_gettime(libc::CLOCK_MONOTONIC, &mut value) };
    if status != 0 {
        return Err(std::io::Error::last_os_error().into());
    }
    let seconds = u128::try_from(value.tv_sec).context(DEADLINE_NANOSECONDS_INVALID)?;
    let nanoseconds = u128::try_from(value.tv_nsec).context(DEADLINE_NANOSECONDS_INVALID)?;
    ensure!(
        nanoseconds < crate::time_constants::NANOSECONDS_PER_SECOND,
        DEADLINE_NANOSECONDS_INVALID
    );
    seconds
        .checked_mul(crate::time_constants::NANOSECONDS_PER_SECOND)
        .and_then(|base| base.checked_add(nanoseconds))
        .context(DEADLINE_OVERFLOW)
}

fn monotonic_anchor() -> Result<MonotonicAnchor> {
    if let Some(anchor) = MONOTONIC_ANCHOR.get() {
        return Ok(*anchor);
    }
    let anchor = select_monotonic_anchor(monotonic_nanoseconds)?;
    let _ = MONOTONIC_ANCHOR.set(anchor);
    Ok(*MONOTONIC_ANCHOR.get().context(DEADLINE_BOOT_UNKNOWN)?)
}

fn select_monotonic_anchor(
    mut read_nanoseconds: impl FnMut() -> Result<u128>,
) -> Result<MonotonicAnchor> {
    let mut selected: Option<MonotonicAnchor> = None;
    for _ in 0..MONOTONIC_ANCHOR_SAMPLE_ATTEMPTS {
        let candidate = MonotonicAnchor {
            before_nanoseconds: read_nanoseconds()?,
            instant: tokio::time::Instant::now(),
            after_nanoseconds: read_nanoseconds()?,
        };
        let candidate_skew = candidate
            .after_nanoseconds
            .checked_sub(candidate.before_nanoseconds)
            .context(DEADLINE_NANOSECONDS_INVALID)?;
        let selected_skew = selected
            .map(|value| value.after_nanoseconds - value.before_nanoseconds)
            .unwrap_or(u128::MAX);
        if candidate_skew < selected_skew {
            selected = Some(candidate);
        }
    }
    let selected = selected.context(DEADLINE_ANCHOR_SKEW)?;
    ensure!(
        selected.after_nanoseconds - selected.before_nanoseconds
            <= MONOTONIC_ANCHOR_MAX_SKEW.as_nanos(),
        DEADLINE_ANCHOR_SKEW
    );
    Ok(selected)
}

#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-unit-tests"))]
#[path = "../../unit_tests/launch/launch_deadline_tests.rs"]
mod tests;
