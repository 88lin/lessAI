const INLINE_VERB_COMMAND: &str = "\\verb";
const LINE_BREAK_COMMAND: &str = "\\\\";

use super::environments::{
    begin_environment_name, is_math_environment_name, is_raw_environment_name,
};

pub(super) fn is_escaped(text: &str, index: usize) -> bool {
    if index == 0 {
        return false;
    }
    let bytes = text.as_bytes();
    let mut backslashes = 0usize;
    let mut pos = index;
    while pos > 0 {
        pos -= 1;
        if bytes[pos] == b'\\' {
            backslashes = backslashes.saturating_add(1);
        } else {
            break;
        }
    }
    backslashes % 2 == 1
}

pub(super) fn find_line_end(text: &str, start: usize) -> usize {
    let bytes = text.as_bytes();
    let mut index = start;
    while index < bytes.len() && bytes[index] != b'\n' && bytes[index] != b'\r' {
        index += 1;
    }
    if index >= bytes.len() {
        return bytes.len();
    }
    if bytes[index] == b'\r' && index + 1 < bytes.len() && bytes[index + 1] == b'\n' {
        return index + 2;
    }
    index + 1
}

pub(super) fn find_substring(text: &str, from: usize, needle: &str) -> Option<usize> {
    text[from..]
        .find(needle)
        .map(|offset| from + offset + needle.len())
}

pub(super) fn find_closing_double_dollar(text: &str, from: usize) -> Option<usize> {
    let bytes = text.as_bytes();
    let mut index = from;
    while index + 1 < bytes.len() {
        if bytes[index] == b'$' && bytes[index + 1] == b'$' && !is_escaped(text, index) {
            return Some(index + 2);
        }
        index += 1;
    }
    None
}

pub(super) fn find_closing_single_dollar(text: &str, from: usize) -> Option<usize> {
    let bytes = text.as_bytes();
    let mut index = from;
    while index < bytes.len() {
        if bytes[index] == b'$' && !is_escaped(text, index) {
            return Some(index + 1);
        }
        index += 1;
    }
    None
}

pub(super) fn find_skip_environment_span(text: &str, index: usize) -> Option<(usize, usize)> {
    let env_name = begin_environment_name(&text[index..])?;
    if !is_locked_environment_name(env_name) {
        return None;
    }

    let span_start = adjust_to_line_start_if_only_whitespace(text, index, 0);
    let pattern = format!("\\end{{{env_name}}}");
    let search_from = index + "\\begin{".len() + env_name.len() + 1;
    let close_start = text[search_from..]
        .find(&pattern)
        .map(|offset| search_from + offset);
    let span_end = match close_start {
        Some(pos) => find_line_end(text, pos + pattern.len()),
        None => text.len(),
    };

    Some((span_start, span_end))
}

pub(super) fn find_inline_verb_end(text: &str, index: usize) -> Option<usize> {
    if !text[index..].starts_with(INLINE_VERB_COMMAND) {
        return None;
    }

    let bytes = text.as_bytes();
    let mut pos = index + INLINE_VERB_COMMAND.len();
    if pos < bytes.len() && bytes[pos] == b'*' {
        pos += 1;
    }
    if pos >= bytes.len() {
        return None;
    }

    let delim = bytes[pos] as char;
    if delim.is_whitespace() {
        return None;
    }
    pos += 1;
    while pos < bytes.len() {
        if bytes[pos] as char == delim {
            return Some(pos + 1);
        }
        pos += 1;
    }
    Some(bytes.len())
}

pub(super) fn find_inline_delimited_command_end(
    text: &str,
    index: usize,
    command: &str,
) -> Option<usize> {
    if !text[index..].starts_with(command) {
        return None;
    }

    let bytes = text.as_bytes();
    let mut pos = index + command.len();
    if pos < bytes.len() && bytes[pos] == b'*' {
        pos += 1;
    }

    loop {
        pos = consume_whitespace(text, pos);
        if pos >= bytes.len() {
            return None;
        }
        if bytes[pos] == b'[' {
            pos = parse_bracket_group(text, pos)?;
            continue;
        }
        break;
    }

    pos = consume_whitespace(text, pos);
    if pos >= bytes.len() {
        return None;
    }
    let delim = bytes[pos];
    if delim.is_ascii_whitespace() || matches!(delim, b'{' | b'}') {
        return None;
    }

    pos += 1;
    while pos < bytes.len() {
        if bytes[pos] == delim {
            return Some(pos + 1);
        }
        pos += 1;
    }
    Some(bytes.len())
}

pub(super) fn find_command_span_end(text: &str, index: usize) -> Option<usize> {
    if !text[index..].starts_with('\\') {
        return None;
    }
    let bytes = text.as_bytes();
    let mut pos = index + 1;
    if pos >= bytes.len() {
        return Some(index + 1);
    }

    if bytes[pos].is_ascii_alphabetic() {
        while pos < bytes.len() && bytes[pos].is_ascii_alphabetic() {
            pos += 1;
        }
        if pos < bytes.len() && bytes[pos] == b'*' {
            pos += 1;
        }
    } else {
        pos += 1;
        if text[index..].starts_with(LINE_BREAK_COMMAND) {
            if pos < bytes.len() && bytes[pos] == b'*' {
                pos += 1;
            }
            pos = consume_whitespace(text, pos);
            if pos < bytes.len() && bytes[pos] == b'[' {
                pos = parse_bracket_group(text, pos).unwrap_or(bytes.len());
            }
        }
        return Some(pos);
    }

    loop {
        let after_ws = consume_whitespace(text, pos);
        if after_ws >= bytes.len() {
            break;
        }
        if bytes[after_ws] == b'[' {
            pos = parse_bracket_group(text, after_ws).unwrap_or(bytes.len());
            continue;
        }
        if bytes[after_ws] == b'{' {
            pos = parse_brace_group(text, after_ws).unwrap_or(bytes.len());
            continue;
        }
        break;
    }

    Some(pos)
}

pub(super) fn consume_whitespace(text: &str, mut pos: usize) -> usize {
    let bytes = text.as_bytes();
    while pos < bytes.len() && matches!(bytes[pos], b' ' | b'\t' | b'\n' | b'\r') {
        pos += 1;
    }
    pos
}

pub(super) fn parse_bracket_group(text: &str, start: usize) -> Option<usize> {
    let bytes = text.as_bytes();
    if start >= bytes.len() || bytes[start] != b'[' {
        return None;
    }
    let mut pos = start + 1;
    while pos < bytes.len() {
        match bytes[pos] {
            b'\\' => pos = (pos + 2).min(bytes.len()),
            b']' => return Some(pos + 1),
            _ => pos += 1,
        }
    }
    Some(bytes.len())
}

pub(super) fn parse_brace_group(text: &str, start: usize) -> Option<usize> {
    let bytes = text.as_bytes();
    if start >= bytes.len() || bytes[start] != b'{' {
        return None;
    }

    let mut depth = 1usize;
    let mut pos = start + 1;
    while pos < bytes.len() {
        match bytes[pos] {
            b'\\' => pos = (pos + 2).min(bytes.len()),
            b'{' => {
                depth = depth.saturating_add(1);
                pos += 1;
            }
            b'}' => {
                depth = depth.saturating_sub(1);
                pos += 1;
                if depth == 0 {
                    return Some(pos);
                }
            }
            _ => pos += 1,
        }
    }
    Some(bytes.len())
}

fn is_locked_environment_name(name: &str) -> bool {
    is_raw_environment_name(name) || is_math_environment_name(name)
}

fn find_line_start(text: &str, index: usize) -> usize {
    let bytes = text.as_bytes();
    let mut pos = index.min(bytes.len());
    while pos > 0 {
        let prev = pos - 1;
        if bytes[prev] == b'\n' || bytes[prev] == b'\r' {
            break;
        }
        pos -= 1;
    }
    pos
}

fn adjust_to_line_start_if_only_whitespace(text: &str, index: usize, lower_bound: usize) -> usize {
    let line_start = find_line_start(text, index);
    if line_start < lower_bound {
        return index;
    }
    if text[line_start..index].trim().is_empty() {
        return line_start;
    }
    index
}
