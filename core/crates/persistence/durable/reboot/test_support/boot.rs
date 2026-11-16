const MACOS_BOOT_PREFIX: &str = "macos:";
const LINUX_BOOT_PREFIX: &str = "durable-reboot-typed-boot-";
pub(super) const SOURCE_BOOT: u64 = 1;
pub(super) const TARGET_BOOT: u64 = 2;

pub(super) fn identity(discriminator: u64) -> String {
    if cfg!(target_os = "macos") {
        format!(
            "{MACOS_BOOT_PREFIX}{}",
            uuid::Uuid::from_u128(u128::from(discriminator))
        )
    } else {
        format!("{LINUX_BOOT_PREFIX}{discriminator}")
    }
}
