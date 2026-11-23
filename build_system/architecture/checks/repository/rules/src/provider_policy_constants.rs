//! Provider policy literals and retired paths for the repository rule.
//! Centralized here so the literal policy treats them as definitions.
pub(crate) const PROVIDERS: &[&str] = &["claude", "codex", "gemini", "grok", "qwen"];

pub(crate) const POLICY_LITERALS: &[&str] = &[
    "codex",
    "claude",
    "qwen",
    "gemini",
    "grok",
    "claude_code",
    "qwen_code",
    "gemini_cli",
    "Codex",
    "Claude Code",
    "Qwen Code",
    "Gemini CLI",
    "Grok Build",
    "yolo.provider.codex",
    "yolo.provider.claude-code",
    "yolo.provider.qwen-code",
    "yolo.provider.gemini-cli",
    "yolo.provider.grok",
];

pub(crate) const FORBIDDEN_PREFIXES: &[&str] = &[
    "core/crates/agent/system/hooks/installation/catalog/",
    "core/crates/conversation/providers/",
    "core/crates/agent/system/providers/src/builtin/",
];

pub(crate) const FORBIDDEN_FILES: &[&str] = &[
    "core/crates/agent/system/providers/src/provider_constants.rs",
    "core/crates/agent/system/providers/src/turn_end.rs",
    "core/crates/plugin/provider/src/constants.rs",
];

pub(crate) const COMPOSITION_FILES: &[&str] = &[
    "core/crates/agent/system/providers/src/builtin.rs",
    "core/crates/plugin/provider/catalog/src/lib.rs",
];

pub(crate) const WIRE_CONSTANT_FILES: &[&str] = &[
    "core/crates/agent/contract/src/constants.rs",
    "core/crates/agent/hooks/src/constants.rs",
    "core/crates/hub/agent/protocol/src/constants.rs",
    "core/crates/conversation/protocol/src/constants.rs",
];

pub(crate) const RETIRED_PATH_MESSAGE: &str = "retired provider policy path must not exist";
pub(crate) const RETIRED_PROVIDER_MESSAGE: &str =
    "retired provider location; implementation must live under agent/providers";
pub(crate) const LITERAL_MESSAGE: &str = "provider policy literal must live in the provider package";
