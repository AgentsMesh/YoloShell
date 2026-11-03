/// Presentation text is a single line without control or directional overrides.
pub fn safe_character(character: char) -> bool {
    !character.is_control()
        && !matches!(character, '\u{061c}' | '\u{200e}' | '\u{200f}' | '\u{202a}'..='\u{202e}' | '\u{2066}'..='\u{2069}')
}

pub fn valid_text(text: &str, maximum: usize) -> bool {
    text.len() <= maximum && text.chars().all(safe_character)
}
