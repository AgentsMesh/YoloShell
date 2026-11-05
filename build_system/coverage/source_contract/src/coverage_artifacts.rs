// These are the actual LLVM tools and companion libraries in the pinned
// rules_rust distribution. A toolchain upgrade must review its artifact set.
const LINUX_DISTRIBUTIONS: &[&str] = &[
    "external/rules_rust++rust+rust_linux_aarch64__aarch64-unknown-linux-gnu__stable_tools/lib/rustlib/aarch64-unknown-linux-gnu/",
    "external/rules_rust++rust+rust_linux_x86_64__x86_64-unknown-linux-gnu__stable_tools/lib/rustlib/x86_64-unknown-linux-gnu/",
];
const MACOS_DISTRIBUTIONS: &[&str] = &[
    "external/rules_rust++rust+rust_macos_aarch64__aarch64-apple-darwin__stable_tools/lib/rustlib/aarch64-apple-darwin/",
    "external/rules_rust++rust+rust_macos_x86_64__x86_64-apple-darwin__stable_tools/lib/rustlib/x86_64-apple-darwin/",
];
const TOOLS: &[&str] = &["bin/llvm-cov", "bin/llvm-profdata"];
const LINUX_LIBRARIES: &[&str] = &[
    "lib/libLLVM-21-rust-1.93.1-stable.so",
    "lib/libLLVM.so.21.1-rust-1.93.1-stable",
];

pub(crate) fn pinned_distribution(source: &str) -> bool {
    LINUX_DISTRIBUTIONS.iter().any(|directory| {
        source.strip_prefix(directory).is_some_and(|relative| {
            TOOLS.contains(&relative) || LINUX_LIBRARIES.contains(&relative)
        })
    }) || MACOS_DISTRIBUTIONS.iter().any(|directory| {
        source
            .strip_prefix(directory)
            .is_some_and(|relative| TOOLS.contains(&relative))
    })
}
