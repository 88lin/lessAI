pub(super) fn count_run(bytes: &[u8], start: usize, target: u8) -> usize {
    let mut len = 0usize;
    let mut index = start;
    while index < bytes.len() && bytes[index] == target {
        len = len.saturating_add(1);
        index += 1;
    }
    len
}

pub(super) fn find_backtick_closing(bytes: &[u8], from: usize, run_len: usize) -> Option<usize> {
    if run_len == 0 {
        return None;
    }
    let mut index = from;
    while index + run_len <= bytes.len() {
        if bytes[index] == b'`' {
            let candidate = count_run(bytes, index, b'`');
            if candidate == run_len {
                return Some(index + run_len);
            }
            index += candidate.max(1);
            continue;
        }
        index += 1;
    }
    None
}

pub(super) fn find_matching_paren(line: &str, start: usize) -> Option<usize> {
    let bytes = line.as_bytes();
    if start >= bytes.len() || bytes[start] != b'(' {
        return None;
    }
    let mut depth = 1usize;
    let mut index = start + 1;
    while index < bytes.len() {
        match bytes[index] {
            b'\\' => index = (index + 2).min(bytes.len()),
            b'(' => {
                depth = depth.saturating_add(1);
                index += 1;
            }
            b')' => {
                depth = depth.saturating_sub(1);
                index += 1;
                if depth == 0 {
                    return Some(index);
                }
            }
            _ => index += 1,
        }
    }
    None
}

pub(super) fn find_matching_bracket(line: &str, start: usize) -> Option<usize> {
    let bytes = line.as_bytes();
    if start >= bytes.len() || bytes[start] != b'[' {
        return None;
    }
    let mut depth = 1usize;
    let mut index = start + 1;
    while index < bytes.len() {
        match bytes[index] {
            b'\\' => index = (index + 2).min(bytes.len()),
            b'[' => {
                depth = depth.saturating_add(1);
                index += 1;
            }
            b']' => {
                depth = depth.saturating_sub(1);
                index += 1;
                if depth == 0 {
                    return Some(index);
                }
            }
            _ => index += 1,
        }
    }
    None
}

pub(super) fn is_markdown_escaped(bytes: &[u8], index: usize) -> bool {
    if index == 0 || index > bytes.len() {
        return false;
    }
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

pub(super) fn find_markdown_math_span_end(line: &str, start: usize) -> Option<usize> {
    let bytes = line.as_bytes();
    if start >= bytes.len() || bytes[start] != b'$' || is_markdown_escaped(bytes, start) {
        return None;
    }

    let delimiter_len = if start + 1 < bytes.len() && bytes[start + 1] == b'$' {
        2usize
    } else {
        1usize
    };

    let mut index = start + delimiter_len;
    while index < bytes.len() {
        if bytes[index] != b'$' {
            index += 1;
            continue;
        }
        if is_markdown_escaped(bytes, index) {
            index += 1;
            continue;
        }

        if delimiter_len == 2 {
            if index + 1 < bytes.len() && bytes[index + 1] == b'$' {
                return (index > start + delimiter_len).then_some(index + 2);
            }
            index += 1;
            continue;
        }

        return (index > start + delimiter_len).then_some(index + 1);
    }

    None
}
