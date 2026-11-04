use yolo_hook_installation::HookInstallSpec;
use yolo_runtime::cli::{ProcessRole, constants};

#[path = "installed_hook/constants.rs"]
mod values;
pub(super) use values::*;

pub fn hook_command(provider: &HookInstallSpec) -> &'static str {
    provider.hook_command
}

pub fn callback_script(provider: &HookInstallSpec) -> Vec<u8> {
    let prefix = ProcessRole::Host.arguments();
    let expected = prefix.len() + CALLBACK_SUFFIX_ARGUMENTS;
    let mut script = format!("#!/bin/sh\n[ \"$#\" -eq {expected} ] || exit {ARGUMENT_FAILURE}\n");
    for (index, argument) in prefix.iter().enumerate() {
        let position = index + SHELL_FIRST_POSITION;
        script.push_str(&format!(
            "[ \"${{{position}}}\" = '{argument}' ] || exit {ARGUMENT_FAILURE}\n"
        ));
    }
    script.push_str(&format!(
        "shift {}\n[ \"$1\" = '{}' ] && [ \"$2\" = '{}' ] && [ -n \"$3\" ] || exit {ARGUMENT_FAILURE}\nexec /bin/cat > \"$3\"\n",
        prefix.len(), hook_command(provider), constants::HOME_ARGUMENT
    ));
    script.into_bytes()
}
