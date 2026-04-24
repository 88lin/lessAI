use super::inline_scans::{count_run, is_markdown_escaped};

#[derive(Clone, Copy)]
pub(super) struct EmphasisDelimiterRun {
    pub marker: u8,
    pub start: usize,
    pub end: usize,
    pub can_open: bool,
    pub can_close: bool,
}

pub(super) fn parse_emphasis_delimiter_run(
    text: &str,
    start: usize,
) -> Option<EmphasisDelimiterRun> {
    let bytes = text.as_bytes();
    let marker = *bytes.get(start)?;
    if !matches!(marker, b'*' | b'_' | b'~') || is_markdown_escaped(bytes, start) {
        return None;
    }

    let len = count_run(bytes, start, marker);
    if marker == b'~' && len < 2 {
        return None;
    }

    let end = start + len;
    let before = text[..start].chars().next_back();
    let after = text[end..].chars().next();
    let left_flanking = is_left_flanking(before, after);
    let right_flanking = is_right_flanking(before, after);
    let before_is_punctuation = is_markdown_punctuation(before);
    let after_is_punctuation = is_markdown_punctuation(after);

    let (can_open, can_close) = match marker {
        b'*' => (left_flanking, right_flanking),
        b'_' => (
            left_flanking && (!right_flanking || before_is_punctuation),
            right_flanking && (!left_flanking || after_is_punctuation),
        ),
        b'~' => (left_flanking, right_flanking),
        _ => return None,
    };

    Some(EmphasisDelimiterRun {
        marker,
        start,
        end,
        can_open,
        can_close,
    })
}

pub(super) fn find_matching_emphasis(
    text: &str,
    open: EmphasisDelimiterRun,
) -> Option<(usize, usize, usize)> {
    let pair_lengths = emphasis_pair_lengths(open.marker, open.end - open.start);
    for pair_len in pair_lengths.into_iter().flatten() {
        if let Some(close_start) = find_matching_emphasis_close(text, open, pair_len) {
            return Some((pair_len, close_start, pair_len));
        }
    }
    None
}

fn emphasis_pair_lengths(marker: u8, run_len: usize) -> [Option<usize>; 2] {
    match marker {
        b'~' if run_len >= 2 => [Some(2), None],
        b'*' | b'_' if run_len >= 3 => [Some(2), Some(1)],
        b'*' | b'_' if run_len == 2 => [Some(2), None],
        b'*' | b'_' if run_len == 1 => [Some(1), None],
        _ => [None, None],
    }
}

fn find_matching_emphasis_close(
    text: &str,
    open: EmphasisDelimiterRun,
    pair_len: usize,
) -> Option<usize> {
    let bytes = text.as_bytes();
    let mut index = open.end;
    while index < bytes.len() {
        let Some(candidate) = parse_emphasis_delimiter_run(text, index) else {
            index += 1;
            continue;
        };
        if candidate.marker != open.marker {
            index = candidate.end;
            continue;
        }
        if candidate.can_close && candidate.end - candidate.start >= pair_len {
            return Some(close_delimiter_start(candidate, pair_len));
        }
        index = candidate.end;
    }
    None
}

fn close_delimiter_start(run: EmphasisDelimiterRun, pair_len: usize) -> usize {
    let run_len = run.end - run.start;
    if pair_len == 2 && run_len > pair_len {
        return run.end - pair_len;
    }
    run.start
}

fn is_left_flanking(before: Option<char>, after: Option<char>) -> bool {
    let Some(after) = after else {
        return false;
    };
    if after.is_whitespace() {
        return false;
    }
    !is_markdown_punctuation(Some(after))
        || is_markdown_whitespace(before)
        || is_markdown_punctuation(before)
}

fn is_right_flanking(before: Option<char>, after: Option<char>) -> bool {
    let Some(before) = before else {
        return false;
    };
    if before.is_whitespace() {
        return false;
    }
    !is_markdown_punctuation(Some(before))
        || is_markdown_whitespace(after)
        || is_markdown_punctuation(after)
}

fn is_markdown_whitespace(ch: Option<char>) -> bool {
    match ch {
        Some(value) => value.is_whitespace(),
        None => true,
    }
}

fn is_markdown_punctuation(ch: Option<char>) -> bool {
    match ch {
        Some(value) => !value.is_whitespace() && !value.is_alphanumeric(),
        None => false,
    }
}

#[cfg(test)]
mod tests {
    use super::{find_matching_emphasis, parse_emphasis_delimiter_run};

    #[test]
    fn find_matching_emphasis_falls_back_to_single_delimiter_when_double_missing() {
        let text = "***a*";
        let open = parse_emphasis_delimiter_run(text, 0).expect("open delimiter");

        let matched = find_matching_emphasis(text, open).expect("should match single fallback");

        assert_eq!(matched, (1, 4, 1));
    }
}
