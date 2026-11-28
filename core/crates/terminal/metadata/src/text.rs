use crate::constants::OSC_SEPARATOR;
use yolo_terminal_metadata_protocol::safe_character;

pub(crate) fn sanitize(parts: &[&[u8]], maximum: usize) -> String {
    let mut result = String::new();
    for (index, part) in parts.iter().enumerate() {
        if index > usize::default() && !append(&mut result, OSC_SEPARATOR, maximum) {
            break;
        }
        for character in String::from_utf8_lossy(part)
            .chars()
            .filter(|c| safe_character(*c))
        {
            if !append(&mut result, character, maximum) {
                return result;
            }
        }
    }
    result
}

fn append(result: &mut String, character: char, maximum: usize) -> bool {
    if result.len().saturating_add(character.len_utf8()) > maximum {
        return false;
    }
    result.push(character);
    true
}
