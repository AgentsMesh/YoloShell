use std::collections::BTreeMap;
use yolo_coverage_protocol::MetricKind;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Metric {
    pub found: u64,
    pub hit: u64,
}

impl Metric {
    pub fn checked_add(self, other: Self, context: &str) -> Result<Self, String> {
        Ok(Self {
            found: self
                .found
                .checked_add(other.found)
                .ok_or_else(|| format!("{context} denominator overflow"))?,
            hit: self
                .hit
                .checked_add(other.hit)
                .ok_or_else(|| format!("{context} covered count overflow"))?,
        })
    }

    pub fn validate(self, context: &str) -> Result<Self, String> {
        if self.hit > self.found {
            return Err(format!("{context} covered count exceeds denominator"));
        }
        Ok(self)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Instance {
    pub column: u64,
    pub identity: String,
    pub line: u64,
    pub name: String,
    pub signature: String,
    pub source: String,
    pub vectors: BTreeMap<MetricKind, Vec<u64>>,
    pub branch_found: u64,
}

impl Instance {
    pub fn metric(&self, kind: MetricKind) -> Result<Metric, String> {
        let counts = self
            .vectors
            .get(&kind)
            .ok_or_else(|| format!("missing {} vector for {}", kind.label(), self.name))?;
        let found = if kind == MetricKind::Branches {
            self.branch_found
        } else {
            u64::try_from(counts.len()).map_err(|_| "metric denominator overflow".to_owned())?
        };
        let hit = u64::try_from(counts.iter().filter(|count| **count > 0).count())
            .map_err(|_| "metric covered count overflow".to_owned())?;
        Metric { found, hit }.validate(kind.label())
    }
}

pub type InstanceMap = BTreeMap<String, Vec<Instance>>;
pub type SummaryMap = BTreeMap<String, BTreeMap<MetricKind, Metric>>;
