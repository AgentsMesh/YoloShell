use super::*;

pub(super) fn inline_string(file: &SourceFile, line: usize, value: &str) -> bool {
    literal(value) || numeric_word(value) && command(&unquoted_code(line_at(&file.text, line)))
}

pub(super) fn command(value: &str) -> bool {
    let words = shell_words(value);
    words.iter().any(|word| command_name(word) == "chmod")
        || words
            .windows(2)
            .any(|pair| command_name(pair[0]) == "mkdir" && pair[1] == "-m")
}

fn literal(value: &str) -> bool {
    let words = shell_words(value);
    words
        .windows(2)
        .any(|pair| command_name(pair[0]) == "chmod" && numeric_word(pair[1]))
        || words.windows(3).any(|parts| {
            command_name(parts[0]) == "mkdir" && parts[1] == "-m" && numeric_word(parts[2])
        })
}

fn shell_words(value: &str) -> Vec<&str> {
    value
        .split_ascii_whitespace()
        .map(|word| word.trim_matches(|character: char| "&;()[]{}'\"".contains(character)))
        .collect()
}

fn command_name(word: &str) -> &str {
    word.rsplit('/').next().unwrap_or(word)
}
