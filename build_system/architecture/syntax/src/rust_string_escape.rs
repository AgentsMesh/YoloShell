const ESCAPE: char = '\\';
const UNICODE_OPEN: char = '{';
const UNICODE_CLOSE: char = '}';
const HEX_RADIX: u32 = 16;
const BYTE_HEX_DIGITS: usize = 2;
const DIGIT_SEPARATOR: char = '_';

pub fn decode(source: &str) -> String {
    let mut output = String::with_capacity(source.len());
    let mut characters = source.chars().peekable();
    while let Some(character) = characters.next() {
        if character != ESCAPE {
            output.push(character);
            continue;
        }
        let Some(escaped) = characters.next() else {
            output.push(ESCAPE);
            break;
        };
        match escaped {
            '0' => output.push('\0'),
            'n' => output.push('\n'),
            'r' => output.push('\r'),
            't' => output.push('\t'),
            '\\' => output.push('\\'),
            '\'' => output.push('\''),
            '"' => output.push('"'),
            'x' => decode_byte(&mut characters, &mut output),
            'u' => decode_unicode(&mut characters, &mut output),
            '\n' => skip_continuation(&mut characters),
            other => {
                output.push(ESCAPE);
                output.push(other);
            }
        }
    }
    output
}

fn decode_byte<I>(characters: &mut I, output: &mut String)
where
    I: Iterator<Item = char>,
{
    let digits: String = characters.take(BYTE_HEX_DIGITS).collect();
    if let Ok(value) = u8::from_str_radix(&digits, HEX_RADIX) {
        output.push(char::from(value));
    } else {
        output.push_str("\\x");
        output.push_str(&digits);
    }
}

fn decode_unicode<I>(characters: &mut std::iter::Peekable<I>, output: &mut String)
where
    I: Iterator<Item = char>,
{
    if characters.next() != Some(UNICODE_OPEN) {
        output.push_str("\\u");
        return;
    }
    let digits: String = characters
        .by_ref()
        .take_while(|character| *character != UNICODE_CLOSE)
        .collect();
    let normalized: String = digits
        .chars()
        .filter(|character| *character != DIGIT_SEPARATOR)
        .collect();
    let decoded = u32::from_str_radix(&normalized, HEX_RADIX)
        .ok()
        .and_then(char::from_u32);
    if let Some(character) = decoded {
        output.push(character);
    } else {
        output.push_str("\\u{");
        output.push_str(&digits);
        output.push(UNICODE_CLOSE);
    }
}

fn skip_continuation<I>(characters: &mut std::iter::Peekable<I>)
where
    I: Iterator<Item = char>,
{
    while characters
        .peek()
        .is_some_and(|character| character.is_whitespace())
    {
        characters.next();
    }
}
