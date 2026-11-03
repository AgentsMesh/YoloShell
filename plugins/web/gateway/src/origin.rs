use crate::MAX_ORIGIN_BYTES;
use anyhow::{Result, ensure};

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct WebOrigin(String);

impl WebOrigin {
    pub fn parse(value: impl Into<String>) -> Result<Self> {
        let value = value.into();
        ensure!(value.len() <= MAX_ORIGIN_BYTES, "invalid web origin");
        let url = url::Url::parse(&value).map_err(|_| anyhow::anyhow!("invalid web origin"))?;
        ensure!(
            matches!(url.scheme(), "http" | "https") && url.host_str().is_some(),
            "invalid web origin"
        );
        let canonical = url.origin().ascii_serialization();
        ensure!(
            value.eq_ignore_ascii_case(&canonical),
            "web origin must contain only scheme and authority"
        );
        Ok(Self(canonical))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}
