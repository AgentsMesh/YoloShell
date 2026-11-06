pub(crate) const CONFIG_HOME_SINGLE_PROVIDER: &str =
    "--config-home requires exactly one provider";
pub(crate) const JSON_REQUIRES_YES: &str = "--json requires --yes to confirm installation";
pub(crate) const NON_INTERACTIVE: &str =
    "cannot confirm on a non-interactive terminal; pass --yes";
pub(crate) const RESTRICTED: &str =
    "hooks are disabled in this provider's configuration; enable them before installing";
pub(crate) const UNKNOWN_PROVIDER: &str = "unknown provider";
pub(crate) const KNOWN_PROVIDERS: &str = "known providers";

pub(crate) const STATUS_HEADER: &str = "PROVIDER\tAGENT\tSTATUS\tTRUST\tFILE";
pub(crate) const STATUS_INSTALLED: &str = "installed";
pub(crate) const STATUS_NOT_INSTALLED: &str = "not installed";
pub(crate) const STATUS_DISABLED: &str = "disabled";
pub(crate) const TRUST_REQUIRED: &str = "required";
pub(crate) const TRUST_NOT_REQUIRED: &str = "-";

pub(crate) const ALREADY_INSTALLED: &str = "All requested hooks are already installed.";
pub(crate) const DRY_RUN: &str = "Dry run: no files were written.";
pub(crate) const CANCELLED: &str = "Hook installation cancelled.";
pub(crate) const MANAGED_NOTE: &str =
    "Yolo runs these hooks only inside Yolo-managed terminals.";
pub(crate) const TRUST_NOTE: &str =
    "review and trust these hooks in the provider's own hook settings; Yolo does not bypass hook trust";
pub(crate) const CONFIRM_PREFIX: &str = "Install hooks for";
pub(crate) const CONFIRM_SUFFIX: &str = "? [y/N] ";
pub(crate) const BACKUP_PREFIX: &str = "  backup: ";
pub(crate) const FILE_PREFIX: &str = "  file:    ";
pub(crate) const STATUS_PREFIX: &str = "  status:  ";
pub(crate) const EVENTS_PREFIX: &str = "  events:  ";
pub(crate) const NOTE_PREFIX: &str = "  note:    ";
pub(crate) const TRUST_PREFIX: &str = "  trust:   ";
pub(crate) const EVENTS_SUFFIX: &str = " managed callbacks";
pub(crate) const INSTALLED_SUFFIX: &str = ": installed at ";
