use anyhow::Result;

#[cfg(target_os = "macos")]
#[path = "discovery_capability_macos.rs"]
mod macos;
#[cfg(target_os = "macos")]
pub use macos::{ProcessDiscoveryInheritedWriter, ProcessDiscoveryWriterInheritance};
#[cfg(target_os = "linux")]
#[path = "discovery_capability_linux.rs"]
mod linux;

const CAPABILITY_RANDOM_BYTES: usize = 32;
const HEX_RADIX: usize = 16;
const HEX_DIGITS_PER_BYTE: usize = 2;
const CAPABILITY_HEX_DIGITS: &[u8; HEX_RADIX] = b"0123456789abcdef";
const BITS_PER_HEX_DIGIT: u32 = 4;
const LOW_HEX_DIGIT_MASK: u8 = 0x0f;
const CAPABILITY_GENERATION_CONTEXT: &str = "generate process discovery capability";
const CAPABILITY_VALUE_DETAIL: &str = "process discovery capability is invalid";

#[derive(Clone)]
pub struct ProcessDiscoveryCapability {
    value: String,
    #[cfg(target_os = "linux")]
    platform: linux::LinuxDiscoveryCapability,
    #[cfg(target_os = "macos")]
    platform: macos::MacosDiscoveryCapability,
}

impl ProcessDiscoveryCapability {
    pub fn generate() -> Result<Self> {
        let mut bytes = [0_u8; CAPABILITY_RANDOM_BYTES];
        getrandom::fill(&mut bytes)
            .map_err(|error| anyhow::anyhow!("{CAPABILITY_GENERATION_CONTEXT}: {error}"))?;
        let value = lowercase_hex(bytes);
        Ok(Self {
            #[cfg(target_os = "linux")]
            platform: linux::LinuxDiscoveryCapability::generated(&value)?,
            #[cfg(target_os = "macos")]
            platform: macos::MacosDiscoveryCapability::generate()?,
            value,
        })
    }

    pub fn value(&self) -> &str {
        &self.value
    }

    #[cfg(not(target_os = "macos"))]
    pub fn configure_child_inheritance(&self, command: &mut std::process::Command) {
        command.env(super::PROCESS_DISCOVERY_CAPABILITY_ENV, &self.value);
    }

    #[cfg(not(target_os = "macos"))]
    pub fn adopt_inherited_manager() -> Result<Self> {
        let value = std::env::var(super::PROCESS_DISCOVERY_CAPABILITY_ENV)?;
        validate_value(&value)?;
        Ok(Self {
            #[cfg(target_os = "linux")]
            platform: linux::LinuxDiscoveryCapability::adopted(&value)?,
            value,
        })
    }

    #[cfg(not(target_os = "macos"))]
    pub(super) fn into_value(self) -> String {
        self.value
    }
}

impl PartialEq for ProcessDiscoveryCapability {
    fn eq(&self, other: &Self) -> bool {
        self.value == other.value
    }
}

impl Eq for ProcessDiscoveryCapability {}

fn lowercase_hex(bytes: [u8; CAPABILITY_RANDOM_BYTES]) -> String {
    let mut value = String::with_capacity(CAPABILITY_RANDOM_BYTES * HEX_DIGITS_PER_BYTE);
    for byte in bytes {
        value.push(CAPABILITY_HEX_DIGITS[usize::from(byte >> BITS_PER_HEX_DIGIT)] as char);
        value.push(CAPABILITY_HEX_DIGITS[usize::from(byte & LOW_HEX_DIGIT_MASK)] as char);
    }
    value
}

fn validate_value(value: &str) -> Result<()> {
    anyhow::ensure!(
        value.len() == CAPABILITY_RANDOM_BYTES * HEX_DIGITS_PER_BYTE
            && value
                .as_bytes()
                .iter()
                .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase()),
        CAPABILITY_VALUE_DETAIL
    );
    Ok(())
}
