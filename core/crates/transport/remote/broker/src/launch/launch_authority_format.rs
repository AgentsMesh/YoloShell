use super::BrokerLaunchAuthority;
use std::str::FromStr;

impl std::fmt::Display for BrokerLaunchAuthority {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(formatter)
    }
}

impl FromStr for BrokerLaunchAuthority {
    type Err = uuid::Error;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        uuid::Uuid::parse_str(value).map(BrokerLaunchAuthority)
    }
}
