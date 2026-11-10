load("@rules_rust//rust:toolchain.bzl", "rust_stdlib_filegroup", "rust_toolchain")

package(default_visibility = ["//visibility:public"])

exports_files(["build-receipt.json"])

filegroup(
    name = "rustc_lib",
    srcs = glob(
        [
            "lib/*.dylib",
            "lib/rustlib/{HOST}/lib/*.dylib",
            "lib/rustlib/{HOST}/lib/*.rmeta",
        ],
        allow_empty = True,
    ),
)

rust_stdlib_filegroup(
    name = "rust_std",
    srcs = glob(["lib/rustlib/{HOST}/lib/*"]),
)

filegroup(
    name = "llvm_lib",
    srcs = glob(
        ["llvm/lib/*.dylib"],
        allow_empty = True,
    ),
)

rust_toolchain(
    name = "compiler",
    # Native C/C++ toolchain supplies the linker. Never borrow a stock Rust sysroot.
    binary_ext = "",
    default_edition = "2024",
    dylib_ext = ".dylib",
    exec_triple = "{HOST}",
    extra_exec_rustc_flags = ["-Zcoverage-direct-counters"],
    extra_rustc_flags = ["-Zcoverage-direct-counters"],
    llvm_cov = "llvm/bin/llvm-cov",
    llvm_lib = ":llvm_lib",
    llvm_profdata = "llvm/bin/llvm-profdata",
    rust_doc = "bin/rustdoc",
    rust_std = ":rust_std",
    rustc = "bin/rustc",
    rustc_lib = ":rustc_lib",
    staticlib_ext = ".a",
    stdlib_linkflags = [
        "-lSystem",
        "-lresolv",
    ],
    target_triple = "{HOST}",
)

toolchain(
    name = "toolchain",
    exec_compatible_with = [
        "@platforms//cpu:aarch64",
        "@platforms//os:osx",
    ],
    target_compatible_with = [
        "@platforms//cpu:aarch64",
        "@platforms//os:osx",
    ],
    toolchain = ":compiler",
    toolchain_type = "@rules_rust//rust:toolchain_type",
)
