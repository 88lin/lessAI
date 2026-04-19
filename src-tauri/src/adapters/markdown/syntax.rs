const MIN_FENCE_MARKER_LEN: usize = 3;
const MAX_ATX_HEADING_LEVEL: usize = 6;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct FenceMarker {
    pub ch: char,
    pub len: usize,
}

pub(super) fn is_yaml_front_matter_open(line: &str) -> bool {
    let trimmed = line.trim_start_matches('\u{feff}').trim();
    trimmed == "---"
}

pub(super) fn is_yaml_front_matter_close(line: &str) -> bool {
    matches!(line.trim(), "---" | "...")
}

pub(super) fn detect_fence_marker(line: &str) -> Option<FenceMarker> {
    let trimmed = line.trim_start();
    let first = trimmed.chars().next()?;
    if first != '`' && first != '~' {
        return None;
    }

    let len = trimmed.chars().take_while(|ch| *ch == first).count();
    (len >= MIN_FENCE_MARKER_LEN).then_some(FenceMarker { ch: first, len })
}

pub(super) fn is_fence_close(line: &str, marker: FenceMarker) -> bool {
    let trimmed = line.trim_start();
    let count = trimmed.chars().take_while(|ch| *ch == marker.ch).count();
    count >= marker.len && trimmed[count..].trim().is_empty()
}

pub(super) fn is_markdown_table_delimiter(line: &str) -> bool {
    let trimmed = line.trim();
    let dash_count = trimmed.chars().filter(|ch| *ch == '-').count();
    !trimmed.is_empty()
        && trimmed.contains('|')
        && dash_count >= MIN_FENCE_MARKER_LEN
        && trimmed
            .chars()
            .all(|ch| matches!(ch, '|' | '-' | ':') || ch.is_whitespace())
}

pub(super) fn is_reference_definition_line(line: &str) -> bool {
    let trimmed = line.trim_start();
    if !trimmed.starts_with('[') {
        return false;
    }
    let close = match trimmed.find("]:") {
        Some(value) => value,
        None => return false,
    };
    close > 1 && !trimmed[close + 2..].trim_start().is_empty()
}

pub(super) fn is_html_like_line(line: &str) -> bool {
    let trimmed = line.trim_start();
    let bytes = trimmed.as_bytes();
    trimmed.starts_with('<')
        && bytes.len() >= 2
        && matches!(bytes[1], b'/' | b'!' | b'?' | b'a'..=b'z' | b'A'..=b'Z')
}

pub(super) fn is_horizontal_rule_line(line: &str) -> bool {
    let trimmed = line.trim();
    let first = match trimmed.chars().next() {
        Some(value) => value,
        None => return false,
    };
    let count = trimmed.chars().filter(|ch| *ch == first).count();
    matches!(first, '-' | '*' | '_')
        && count >= MIN_FENCE_MARKER_LEN
        && trimmed.chars().all(|ch| ch == first || ch.is_whitespace())
}

pub(super) fn is_atx_heading_line(line: &str) -> bool {
    let trimmed = line.trim_start();
    let count = trimmed.chars().take_while(|ch| *ch == '#').count();
    count > 0
        && count <= MAX_ATX_HEADING_LEVEL
        && (trimmed.len() == count || trimmed.as_bytes()[count].is_ascii_whitespace())
}

pub(super) fn is_setext_underline_line(line: &str) -> bool {
    let trimmed = line.trim();
    let mut chars = trimmed.chars();
    let first = match chars.next() {
        Some(value) => value,
        None => return false,
    };
    matches!(first, '-' | '=') && chars.all(|ch| ch == first)
}

pub(super) fn is_indented_code_line(line: &str) -> bool {
    !line.trim().is_empty() && (line.starts_with('\t') || line.starts_with("    "))
}

pub(super) fn is_math_block_delimiter_line(line: &str) -> bool {
    line.trim() == "$$"
}

pub(super) fn is_list_or_quote_line(line: &str) -> bool {
    let trimmed = line.trim_start();
    if trimmed.starts_with('>') {
        return true;
    }
    if trimmed.starts_with("- ") || trimmed.starts_with("* ") || trimmed.starts_with("+ ") {
        return true;
    }

    let bytes = trimmed.as_bytes();
    let mut index = 0usize;
    while index < bytes.len() && bytes[index].is_ascii_digit() {
        index += 1;
    }
    index > 0
        && index + 1 < bytes.len()
        && (bytes[index] == b'.' || bytes[index] == b')')
        && bytes[index + 1].is_ascii_whitespace()
}

pub(super) fn markdown_line_prefix_len(line: &str) -> usize {
    let bytes = line.as_bytes();
    let mut pos = 0usize;
    while pos < bytes.len() && matches!(bytes[pos], b' ' | b'\t') {
        pos += 1;
    }

    let indent_end = pos;
    if indent_end >= bytes.len() {
        return 0;
    }

    if bytes[indent_end] == b'[' && indent_end + 2 < bytes.len() && bytes[indent_end + 1] == b'^' {
        let mut p = indent_end + 2;
        while p < bytes.len() {
            if bytes[p] == b']' {
                break;
            }
            p += 1;
        }
        if p + 1 < bytes.len() && bytes[p] == b']' && bytes[p + 1] == b':' {
            p += 2;
            while p < bytes.len() && bytes[p].is_ascii_whitespace() {
                p += 1;
            }
            return p;
        }
    }

    if bytes[indent_end] == b'#' {
        let mut p = indent_end;
        while p < bytes.len() && bytes[p] == b'#' {
            p += 1;
        }
        let count = p - indent_end;
        if (1..=MAX_ATX_HEADING_LEVEL).contains(&count)
            && p < bytes.len()
            && bytes[p].is_ascii_whitespace()
        {
            while p < bytes.len() && bytes[p].is_ascii_whitespace() {
                p += 1;
            }
            return p;
        }
    }

    if bytes[indent_end] == b'>' {
        let mut p = indent_end;
        while p < bytes.len() && bytes[p] == b'>' {
            p += 1;
        }
        while p < bytes.len() && bytes[p].is_ascii_whitespace() {
            p += 1;
        }
        return p;
    }

    if matches!(bytes[indent_end], b'-' | b'*' | b'+') {
        let mut p = indent_end + 1;
        if p < bytes.len() && bytes[p].is_ascii_whitespace() {
            while p < bytes.len() && bytes[p].is_ascii_whitespace() {
                p += 1;
            }

            if p + 2 < bytes.len() && bytes[p] == b'[' && bytes[p + 2] == b']' {
                let mid = bytes[p + 1];
                if matches!(mid, b' ' | b'x' | b'X') {
                    p += 3;
                    while p < bytes.len() && bytes[p].is_ascii_whitespace() {
                        p += 1;
                    }
                }
            }

            return p;
        }
    }

    let mut p = indent_end;
    while p < bytes.len() && bytes[p].is_ascii_digit() {
        p += 1;
    }
    if p > indent_end
        && p + 1 < bytes.len()
        && matches!(bytes[p], b'.' | b')')
        && bytes[p + 1].is_ascii_whitespace()
    {
        p += 1;
        while p < bytes.len() && bytes[p].is_ascii_whitespace() {
            p += 1;
        }
        return p;
    }

    0
}
