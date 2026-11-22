pub const PROVIDER_DESCRIPTOR_SCHEMA_VERSION: u16 = 1;
pub const MAX_PROVIDER_MANIFEST_ID_BYTES: usize = 96;
pub const MAX_PROVIDER_TOKEN_BYTES: usize = 96;
pub const MAX_PROVIDER_DISPLAY_NAME_BYTES: usize = 96;
pub const MAX_PROVIDER_ICON_BYTES: usize = 96;

pub mod fields {
    pub const SCHEMA_VERSION: &str = "schema_version";
}

pub mod message {
    pub const MANIFEST_ID: &str = "provider manifest id is invalid";
    pub const KIND: &str = "provider agent kind is invalid";
    pub const TOKEN: &str = "provider token is invalid";
    pub const DISPLAY_NAME: &str = "provider display name is invalid";
    pub const ICON: &str = "provider icon is invalid";
    pub const CAPABILITY_FLOOR: &str = "provider target tier exceeds its capabilities";
    pub const PROJECTION: &str = "provider projection policy requires lifecycle capability";
}
