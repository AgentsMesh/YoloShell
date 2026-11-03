# Claude Code schema fixture

`claude-code-2.1.222-transcript.redacted.jsonl` records the JSON keys and JSON value types observed from a local Claude Code 2.1.222 installation.

Only the version and schema discriminants needed to exercise parser behavior are retained. Session identifiers, UUIDs, paths, timestamps, prompts, thinking, tool input, tool output, model output, titles, settings, request identifiers, signatures, branch names, and model names are synthetic redaction sentinels. The fixture intentionally retains selected additional fields so tests prove that Yolo remains permissive when Claude Code adds transcript fields.
