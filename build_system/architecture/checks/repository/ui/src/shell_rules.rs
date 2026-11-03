use yolo_architecture_foundation::SourceFile;
use yolo_architecture_foundation::model::Violation;

const GUI_COMMAND_PARTS: &[&[&str]] = &[&["op", "en"], &["osa", "script"], &["launch", "c", "tl"]];
const COMMAND_PREFIX_PARTS: &[&[&str]] = &[&["com", "mand"], &["env"], &["exec"]];
const CONTROL_WORDS: &[&str] = &["!", "do", "elif", "if", "then", "until", "while"];
const BUNDLE_AUDIT: &str = "plugins/macos/Tests/E2E/bundle_audit_test.sh";
const BUNDLE_POLICY_CALL: &str = "enforce_native_bundle_policy ";

pub fn inspect(file: &SourceFile, violations: &mut Vec<Violation>) {
    inspect_bundle_policy_call(file, violations);
    if !test_shell(&file.relative) {
        return;
    }
    for (index, line) in file.text.lines().enumerate() {
        if split_commands(line)
            .iter()
            .any(|segment| forbidden(segment))
        {
            violations.push(Violation::line(
                &file.relative,
                index + 1,
                "default shell tests must not launch or activate a native application",
            ));
        }
    }
}

fn inspect_bundle_policy_call(file: &SourceFile, violations: &mut Vec<Violation>) {
    if file.relative == BUNDLE_AUDIT
        && !file
            .text
            .lines()
            .map(str::trim_start)
            .any(|line| line.starts_with(BUNDLE_POLICY_CALL))
    {
        violations.push(Violation::file(
            &file.relative,
            "bundle audit must enforce the native payload and dependency policy",
        ));
    }
}

fn test_shell(path: &str) -> bool {
    path.ends_with(".sh")
        && (path.starts_with("plugins/macos/Tests/") || path.starts_with("integration_tests/"))
}

fn forbidden(segment: &str) -> bool {
    command_name(segment).is_some_and(|name| {
        GUI_COMMAND_PARTS
            .iter()
            .any(|parts| name.eq_ignore_ascii_case(&parts.concat()))
    })
}

pub fn command_name(segment: &str) -> Option<&str> {
    let words = segment.split_ascii_whitespace().collect::<Vec<_>>();
    let mut index = 0;
    while index < words.len() {
        let word = clean(words[index]);
        if word.is_empty()
            || CONTROL_WORDS.contains(&word)
            || assignment(word)
            || word.starts_with('-')
        {
            index += 1;
            continue;
        }
        if COMMAND_PREFIX_PARTS
            .iter()
            .any(|parts| word == parts.concat())
        {
            index += 1;
            continue;
        }
        return Some(word.rsplit('/').next().unwrap_or(word));
    }
    None
}

fn clean(word: &str) -> &str {
    word.trim_matches(|character| matches!(character, '\'' | '"' | '\\'))
}

fn assignment(word: &str) -> bool {
    let Some((name, _)) = word.split_once('=') else {
        return false;
    };
    !name.is_empty()
        && name
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
}

pub fn split_commands(line: &str) -> Vec<String> {
    let mut result = Vec::new();
    let mut current = String::new();
    let mut quote = None;
    let mut escaped = false;
    for character in line.chars() {
        if escaped {
            current.push(character);
            escaped = false;
            continue;
        }
        if character == '\\' && quote != Some('\'') {
            current.push(character);
            escaped = true;
            continue;
        }
        if let Some(delimiter) = quote {
            current.push(character);
            if character == delimiter {
                quote = None;
            }
            continue;
        }
        if character == '\'' || character == '"' {
            quote = Some(character);
            current.push(character);
        } else if character == '#' {
            break;
        } else if matches!(character, ';' | '|' | '&' | '(' | ')' | '{' | '}') {
            push_segment(&mut result, &mut current);
        } else {
            current.push(character);
        }
    }
    push_segment(&mut result, &mut current);
    result
}

fn push_segment(result: &mut Vec<String>, current: &mut String) {
    if !current.trim().is_empty() {
        result.push(std::mem::take(current));
    } else {
        current.clear();
    }
}
