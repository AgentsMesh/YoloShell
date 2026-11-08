//! Re-exported provider hook codec contract. The trait lives in
//! `yolo_hook_codec` so provider hook packages depend on the port rather than
//! on the ingress engine; ingress keeps these names for its internal use and
//! for consumers that predate the port split.
pub use yolo_hook_codec::{
    CALLBACK_SHELLS, CanonicalHookEvent, HookAncestry, HookCodec, HookIngressProfile,
};
