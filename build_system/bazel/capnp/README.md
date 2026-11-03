# Cap'n Proto schema compilation

`capnp_rust_library` compiles schemas with the pinned `capnp-cpp` C++ compiler
from Bazel Central Registry and the pinned Rust `capnpc` generator. Both tools
are built for the execution platform. No system `capnp`, Cargo build script,
runtime compiler download, or checked-in generated Rust is used.

```starlark
load("//build_system/bazel/capnp:defs.bzl", "capnp_rust_library")

capnp_rust_library(
    name = "schema",
    srcs = ["value.capnp", "service.capnp"],
    crate_name = "yolo_service_schema",
)
```

The generated crate exports `value_capnp` and `service_capnp`. List every schema
used by the crate in `srcs`; schemas may import each other by local filename.
They live beside their `BUILD.bazel`. Standard system imports are disabled so
undeclared machine state cannot affect the generated wire contract.

For a manually authored crate root, pass `crate_root = "src/lib.rs"` and use:

```rust
pub mod value_capnp {
    include!("generated/value_capnp.rs");
}
```

The compiler writes `src/generated/*_capnp.rs` only under the Bazel output tree;
declare one schema crate per Bazel package. Ordinary Rust targets depend on this
library plus `@crates//:capnp`, or `@crates//:capnp-rpc` for capability RPC.
Async Tokio transports use `@crates//:tokio-util` with its `compat` feature.

The integration target validates native union and UTF-8 serialization, a schema
import, and an actual locally dispatched capability call:

```text
//build_system/bazel/capnp/integration_tests:generation_tests
```
