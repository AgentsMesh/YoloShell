pub(super) const INFERRED_ATTRIBUTES: &[&str] = &[
    "#[serde(default)]",
    "#[serde(default = \"active_profile\")]",
    "#[serde(skip_serializing_if = \"CustodyProfile::is_active_running\")]",
    "#[serde(skip)]",
    "#[serde(skip_deserializing)]",
    "#[serde(skip_serializing)]",
    "#[cfg_attr(feature = \"old_input\", serde(default))]",
    "#[serde(\n default,\n skip_serializing_if = \"is_empty\"\n)]",
];
pub(super) const DENY_ATTRIBUTE: &str = "#[serde(deny_unknown_fields)]";
pub(super) const CONDITIONAL_DENY: &str =
    "#[cfg_attr(feature = \"strict\", serde(deny_unknown_fields))]";
pub(super) const OFFER: &str = r#"
    #[derive(Serialize, Deserialize)]
    #[serde(deny_unknown_fields)]
    pub struct CustodyOffer {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub guardian: Option<Guardian>,
        FIELD_ATTRIBUTES pub(crate) profile: CustodyProfile,
    }
"#;
pub(super) const RECORD: &str = r#"
    #[serde(tag = "kind", deny_unknown_fields)]
    pub enum CustodyDurableState {
        Pending {},
        Armed { ARMED_ATTRIBUTES profile: CustodyProfile, guardian: Option<Guardian> },
        Retired { RETIRED_ATTRIBUTES profile: CustodyProfile, guardian: Option<Guardian> },
    }
    #[serde(deny_unknown_fields)]
    pub struct CustodyDurableLease { LEASE_ATTRIBUTES pub profile: CustodyProfile }
"#;
pub(super) const PROFILE: &str = r#"
    PROFILE_ATTRIBUTES
    pub enum CustodyProfile { ActiveRunning, ReleasedTerminal }
"#;
pub(super) const FIELD_MARKER: &str = "FIELD_ATTRIBUTES";
pub(super) const RECORD_MARKERS: &[&str] =
    &["ARMED_ATTRIBUTES", "RETIRED_ATTRIBUTES", "LEASE_ATTRIBUTES"];
pub(super) const PROFILE_MARKER: &str = "PROFILE_ATTRIBUTES";
pub(super) const PROFILE_DEFAULTS: &[&str] = &[
    "#[derive(Clone, Default, Serialize)]",
    "#[derive(std::default::Default)]",
    "#[cfg_attr(feature = \"old_input\", derive(Default))]",
];
pub(super) const PROFILE_IMPLEMENTATION: &str = "impl std::default::Default for CustodyProfile { fn default() -> Self { Self::ActiveRunning } }";
pub(super) const CURRENT_PROFILE_ATTRIBUTES: &str = "#[derive(Clone, Serialize, Deserialize)]";
pub(super) const IRRELEVANT_DEFAULT: &str = "#[derive(Default)] struct Other;";
pub(super) const EXPLANATION: &str = "// #[serde(default)]\n/* #[derive(Default)] */\n";
pub(super) const WRAPPED_TYPES: &[&str] = &[
    "Option<T>",
    "std::option::Option<T>",
    "Box<T>",
    "Vec<T>",
    "(T,)",
    "&'static T",
];
pub(super) const TYPE_MARKER: &str = "T";
pub(super) const QUALIFIED_PROFILE: &str = "crate::pty_custody::CustodyProfile";
pub(super) const QUALIFIED_PAYLOAD: &str = "::yolo_capnp_value::Value";
