use yolo_architecture_foundation::SourceFile;
use yolo_architecture_foundation::model::Violation;
use yolo_architecture_policy_contracts::constants::{
    PTY_ALIAS_BLOCK_ON_OPERATION, PTY_ALIAS_BLOCKING_ADMISSION, PTY_ALIAS_BLOCKING_LOCK_OPERATION,
    PTY_ALIAS_LOCK_OPERATION, PTY_ALIAS_TRY_LOCK_OPERATION, PTY_ALIAS_YIELD_OPERATION,
};
use yolo_architecture_syntax::rust_source::RustSource;
use yolo_architecture_syntax::targeted_tokens::{Kind, Token};

pub(crate) const CONTROL_ADMISSION_PATH: &str =
    "core/crates/runtime/engine/worker/src/control_admission.rs";
pub(crate) const WORKER_SOURCE_PREFIX: &str = "core/crates/runtime/engine/worker/src/";
pub(crate) const RUST_SOURCE_SUFFIX: &str = ".rs";
pub(crate) const RUNTIME_ALIAS_CUSTODY_PATH: &str =
    "core/crates/runtime/engine/worker/src/pty/alias/pty_runtime_alias_custody.rs";
pub(crate) const RUNTIME_ALIAS_CLOSURE_PATH: &str =
    "core/crates/runtime/engine/worker/src/pty/alias/pty_runtime_alias_closure.rs";
pub(crate) const RUNTIME_ALIAS_CLOSURE_TASK_PATH: &str =
    "core/crates/runtime/engine/worker/src/pty/alias/pty_runtime_alias_closure_task.rs";
pub(crate) const RUNTIME_ALIAS_RELEASE_PATH: &str =
    "core/crates/runtime/engine/worker/src/pty/alias/pty_runtime_alias_release.rs";
pub(crate) const SECONDARY_CUSTODY_EXPORT_PATH: &str =
    "core/crates/runtime/engine/worker/src/secondary_custody/secondary_custody_export.rs";
pub(crate) const PTY_PROCESS_TERMINATION_PATH: &str =
    "core/crates/runtime/engine/worker/src/pty/process/pty_process_termination.rs";
pub(crate) const OUTPUT_RELEASE_PATH: &str =
    "core/crates/runtime/engine/worker/src/pty/output/pty_output_release.rs";
pub(crate) const STARTUP_CUSTODY_PATH: &str =
    "core/crates/runtime/engine/worker/src/pty/output/pty_output_startup_custody.rs";
pub(crate) const STARTUP_RELEASE_PATH: &str =
    "core/crates/runtime/engine/worker/src/pty/output/pty_output_startup_custody_release.rs";
pub(crate) const FORCE_RELEASE_PATH: &str =
    "core/crates/runtime/engine/worker/src/pty/force/pty_force_termination_master_release.rs";
pub(crate) const FORCE_PREPARE_PATH: &str =
    "core/crates/runtime/engine/worker/src/pty/force/pty_force_termination_prepare.rs";
pub(crate) const FORCE_TERMINATION_PATH: &str =
    "core/crates/runtime/engine/worker/src/pty/force/pty_force_termination.rs";
pub(crate) const OWNER_TERMINATION_PATH: &str =
    "core/crates/runtime/engine/worker/src/pty/owner/pty_owner_termination.rs";
pub(crate) const OWNER_PATH: &str = "core/crates/runtime/engine/worker/src/pty/owner/pty_owner.rs";
pub(crate) const BLOCKING_LOCK_OWNED: &str = PTY_ALIAS_BLOCKING_LOCK_OPERATION;
pub(crate) const LOCK_OWNED: &str = PTY_ALIAS_LOCK_OPERATION;
pub(crate) const BLOCK_ON: &str = PTY_ALIAS_BLOCK_ON_OPERATION;
pub(crate) const YIELD_NOW: &str = PTY_ALIAS_YIELD_OPERATION;
pub(crate) const TRY_LOCK_OWNED: &str = PTY_ALIAS_TRY_LOCK_OPERATION;

const WORKER_STATE: &str = "WorkerState";
const BLOCKING_ADMISSION: &str = PTY_ALIAS_BLOCKING_ADMISSION;
const MEMBER_ACCESS: &str = ".";
const CALL_OPEN: &str = "(";
const TOKEN_OFFSET: usize = 1;
const BLOCKING_POLICY: &str = "PTY alias admission permits blocking_lock_owned only in WorkerState::admit_system_mutation_blocking";
const POLLABLE_POLICY: &str = "PTY alias admission and owner master-release paths must remain pollable without blocking or spinning";
const FIFO_POLICY: &str = "PTY alias admission durable closure task must use the typed FIFO mutation admission instead of try_lock_owned polling";

pub(crate) const PROTECTED_PATHS: &[&str] = &[
    RUNTIME_ALIAS_CUSTODY_PATH,
    RUNTIME_ALIAS_CLOSURE_PATH,
    RUNTIME_ALIAS_CLOSURE_TASK_PATH,
    RUNTIME_ALIAS_RELEASE_PATH,
    SECONDARY_CUSTODY_EXPORT_PATH,
    PTY_PROCESS_TERMINATION_PATH,
    OUTPUT_RELEASE_PATH,
    STARTUP_CUSTODY_PATH,
    STARTUP_RELEASE_PATH,
    FORCE_RELEASE_PATH,
    FORCE_PREPARE_PATH,
    FORCE_TERMINATION_PATH,
    OWNER_TERMINATION_PATH,
    OWNER_PATH,
];

pub(crate) const FORBIDDEN_OPERATIONS: &[&str] =
    &[BLOCKING_LOCK_OWNED, LOCK_OWNED, BLOCK_ON, YIELD_NOW];

pub fn inspect(file: &SourceFile, violations: &mut Vec<Violation>) {
    if !file.relative.ends_with(RUST_SOURCE_SUFFIX) {
        return;
    }
    let source = RustSource::parse(&file.text);
    let protected = PROTECTED_PATHS.contains(&file.relative.as_str());
    for (index, token) in source.tokens.iter().enumerate() {
        if token.identifier(TRY_LOCK_OWNED) {
            if file.relative == RUNTIME_ALIAS_CLOSURE_TASK_PATH {
                reject(file, token, FIFO_POLICY, violations);
            }
            continue;
        }
        if token.identifier(BLOCKING_LOCK_OWNED) {
            if !allowed_blocking_admission(file, &source, index) {
                reject(file, token, BLOCKING_POLICY, violations);
            }
            continue;
        }
        if protected && operation(token).is_some() {
            reject(file, token, POLLABLE_POLICY, violations);
        }
    }
}

fn allowed_blocking_admission(file: &SourceFile, source: &RustSource, index: usize) -> bool {
    file.relative == CONTROL_ADMISSION_PATH
        && source
            .function_at(index)
            .is_some_and(|function| function.matches(Some(WORKER_STATE), BLOCKING_ADMISSION))
        && call_syntax(&source.tokens, index)
}

fn call_syntax(tokens: &[Token], index: usize) -> bool {
    index
        .checked_sub(TOKEN_OFFSET)
        .and_then(|previous| tokens.get(previous))
        .is_some_and(|token| token.symbol(MEMBER_ACCESS))
        && tokens
            .get(index + TOKEN_OFFSET)
            .is_some_and(|token| token.symbol(CALL_OPEN))
}

fn operation(token: &Token) -> Option<&str> {
    let Kind::Identifier(identifier) = &token.kind else {
        return None;
    };
    FORBIDDEN_OPERATIONS
        .contains(&identifier.as_str())
        .then_some(identifier)
}

fn reject(file: &SourceFile, token: &Token, message: &str, violations: &mut Vec<Violation>) {
    violations.push(Violation::line(&file.relative, token.line, message));
}
