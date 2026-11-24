pub(super) const FAKE: &str = r#"#!/bin/sh
if [ "$2" = "--version" ]; then
    exit "$PROBE_STATUS"
fi
printf '<%s>\n' "$@"
exit 17
"#;
pub(super) const VERSION: &str = "--version";
pub(super) const SHELL_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(10);
pub(super) const SHELL_POLL: std::time::Duration = std::time::Duration::from_millis(10);
