#[derive(Debug)]
pub struct Segment {
    pub line: usize,
    pub words: Vec<String>,
}

pub fn segments(source: &str) -> Vec<Segment> {
    let mut result = Vec::new();
    let mut words = Vec::new();
    let mut word = String::new();
    let mut quote = None;
    let mut escaped = false;
    let mut line = 1;
    let mut segment_line = line;
    let mut characters = source.chars().peekable();
    while let Some(character) = characters.next() {
        if escaped {
            word.push(character);
            escaped = false;
            continue;
        }
        if character == '\\' && quote != Some('\'') {
            escaped = true;
            continue;
        }
        if let Some(delimiter) = quote {
            if character == delimiter {
                quote = None;
            } else {
                if delimiter == '\'' && character == '$' {
                    word.push('\\');
                }
                word.push(character);
                if character == '\n' {
                    line += 1;
                }
            }
            continue;
        }
        match character {
            '\'' | '"' => quote = Some(character),
            '#' if word.is_empty() => {
                for skipped in characters.by_ref() {
                    if skipped == '\n' {
                        finish_segment(&mut result, &mut words, &mut word, segment_line);
                        line += 1;
                        segment_line = line;
                        break;
                    }
                }
            }
            '\n' => {
                finish_segment(&mut result, &mut words, &mut word, segment_line);
                line += 1;
                segment_line = line;
            }
            ';' | '|' | '&' | '(' | ')' => {
                finish_segment(&mut result, &mut words, &mut word, segment_line);
                segment_line = line;
            }
            value if value.is_whitespace() => push_word(&mut words, &mut word),
            _ => {
                if words.is_empty() && word.is_empty() {
                    segment_line = line;
                }
                word.push(character);
            }
        }
    }
    finish_segment(&mut result, &mut words, &mut word, segment_line);
    result
}

pub fn shell_command_option(option: &str) -> bool {
    let Some(cluster) = option
        .strip_prefix('-')
        .filter(|value| !value.starts_with('-'))
    else {
        return false;
    };
    !cluster.is_empty()
        && cluster.bytes().all(|byte| byte.is_ascii_alphabetic())
        && cluster.contains(SHELL_COMMAND_OPTION)
}

fn finish_segment(
    result: &mut Vec<Segment>,
    words: &mut Vec<String>,
    word: &mut String,
    line: usize,
) {
    push_word(words, word);
    if !words.is_empty() {
        result.push(Segment {
            line,
            words: std::mem::take(words),
        });
    }
}

fn push_word(words: &mut Vec<String>, word: &mut String) {
    if !word.is_empty() {
        words.push(std::mem::take(word));
    }
}
const SHELL_COMMAND_OPTION: char = 'c';
