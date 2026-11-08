use yolo_architecture_foundation::SourceFile;
use yolo_architecture_foundation::model::Violation;
use yolo_architecture_policy_contracts::process_signal as contract;
use yolo_architecture_process_signal_foundation::process_exec_contract as exec_contract;
use yolo_architecture_process_signal_foundation::process_signal_sites::{
    PIDFD_SIGNAL_SITE, Site, TGKILL_SITE, site,
};
use yolo_architecture_process_signal_foundation::process_signal_syscall_syntax as syntax;
use yolo_architecture_source_policy::source_policy::is_rust_test_source;
use yolo_architecture_syntax::rust_source::{RustSource, identifier};
const GETTID_ARGUMENTS: usize = 1;
const PIDFD_OPEN_ARGUMENTS: usize = 3;
const PIDFD_SIGNAL_ARGUMENTS: usize = 5;
const TGKILL_ARGUMENTS: usize = 4;

const GETTID_SITE: Site = site(
    contract::BOOTSTRAP_OBSERVER_PATH,
    None,
    contract::CURRENT_EXEC_THREAD_ID,
);
const PIDFD_OPEN_SITE: Site = site(
    contract::PIDFD_KERNEL_PATH,
    Some(contract::PIDFD_OPERATIONS),
    contract::PIDFD_OPEN_OWNER,
);
struct SyscallSite {
    name: &'static str,
    arguments: usize,
    site: Site,
    no_signal: bool,
}

const SYSCALL_SITES: &[SyscallSite] = &[
    syscall(
        contract::GETTID_SYSCALL,
        GETTID_ARGUMENTS,
        GETTID_SITE,
        false,
    ),
    syscall(
        contract::PIDFD_OPEN_SYSCALL,
        PIDFD_OPEN_ARGUMENTS,
        PIDFD_OPEN_SITE,
        false,
    ),
    syscall(
        contract::PIDFD_SIGNAL_SYSCALL,
        PIDFD_SIGNAL_ARGUMENTS,
        PIDFD_SIGNAL_SITE,
        false,
    ),
    syscall(
        contract::TGKILL_SYSCALL,
        TGKILL_ARGUMENTS,
        TGKILL_SITE,
        true,
    ),
];

pub fn inspect(file: &SourceFile, source: &RustSource, violations: &mut Vec<Violation>) {
    if is_rust_test_source(&file.relative) {
        return;
    }
    for (index, token) in source.tokens.iter().enumerate() {
        if !token.identifier(contract::LIBC_SYSCALL) || unit_test_function(source, index) {
            continue;
        }
        let referenced = syntax::libc_reference(&source.tokens, index);
        let macro_argument = syntax::macro_argument(&source.tokens, index);
        if (referenced || macro_argument)
            && (macro_argument
                || !syntax::libc_call(&source.tokens, index)
                || !approved_call(file, source, index))
        {
            let message = syntax::call_name(&source.tokens, index)
                .map_or(contract::RAW_SIGNAL_MESSAGE, syscall_message);
            violations.push(Violation::line(&file.relative, token.line, message));
        }
    }
    for (index, token) in source.tokens.iter().enumerate() {
        let Some(name) = identifier(token).filter(|name| managed_constant(name)) else {
            continue;
        };
        if unit_test_function(source, index)
            || syntax::containing_call(&source.tokens, index)
                .is_some_and(|call| approved_call(file, source, call))
        {
            continue;
        }
        violations.push(Violation::line(
            &file.relative,
            token.line,
            syscall_message(name),
        ));
    }
}

fn approved_call(file: &SourceFile, source: &RustSource, call: usize) -> bool {
    let Some(name) = syntax::call_name(&source.tokens, call) else {
        return false;
    };
    let Some(arguments) = syntax::arguments(&source.tokens, call) else {
        return false;
    };
    SYSCALL_SITES.iter().any(|entry| {
        entry.name == name
            && entry.arguments == arguments.len()
            && entry
                .site
                .matches(&file.relative, source.function_at(call))
            && (!entry.no_signal
                || arguments.last().is_some_and(|argument| {
                    matches!(*argument, [token] if token.identifier(contract::NO_SIGNAL_CONSTANT))
                }))
    })
}

fn syscall_message(name: &str) -> &'static str {
    if name == exec_contract::EXECVEAT_SYSCALL {
        exec_contract::RAW_EXEC_MESSAGE
    } else if name == contract::PIDFD_SIGNAL_SYSCALL {
        contract::PIDFD_SIGNAL_MESSAGE
    } else {
        contract::RAW_SIGNAL_MESSAGE
    }
}

fn managed_constant(name: &str) -> bool {
    SYSCALL_SITES.iter().any(|entry| entry.name == name)
        || name == exec_contract::EXECVEAT_SYSCALL
        || name == contract::PRCTL_SYSCALL
        || name == contract::TKILL_SYSCALL
        || name == contract::KILL_SYSCALL
        || contract::SIGNAL_QUEUE_SYSCALLS.contains(&name)
}

fn unit_test_function(source: &RustSource, index: usize) -> bool {
    source
        .function_at(index)
        .is_some_and(|function| source.is_unit_test_guarded(function))
}

const fn syscall(name: &'static str, arguments: usize, site: Site, no_signal: bool) -> SyscallSite {
    SyscallSite {
        name,
        arguments,
        site,
        no_signal,
    }
}
