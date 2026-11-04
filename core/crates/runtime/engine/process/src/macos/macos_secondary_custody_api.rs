#[cfg(target_os = "macos")]
pub use macos_secondary_custody::{
    MACOS_SECONDARY_CUSTODY_DESCRIPTOR_COUNT, MacosNamespaceLossContainmentAuthority,
    MacosNamespaceLossFinalized, MacosNamespaceLossSealFinalization, MacosPtyKernelEof,
    MacosSecondaryCustodyDescriptorParts,
};
#[cfg(target_os = "macos")]
pub use macos_secondary_custody::{MacosSecondaryCustody, MacosSecondaryCustodyExport};
