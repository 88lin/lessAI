use crate::rewrite::SegmentedChunk;

const LEFT_BINDING_PUNCTUATION: &[char] = &[
    '，', ',', '、', '：', ':', '；', ';', '。', '.', '！', '!', '？', '?', '）', ')', '】', ']',
    '}', '」', '』', '》', '〉', '”', '’', '"', '\'',
];

pub(super) fn merge_left_binding_punctuation_chunks(
    chunks: Vec<SegmentedChunk>,
) -> Vec<SegmentedChunk> {
    let mut merged: Vec<SegmentedChunk> = Vec::with_capacity(chunks.len());

    for chunk in chunks {
        if try_merge_into_previous(&mut merged, &chunk) {
            continue;
        }
        merged.push(chunk);
    }

    merged
}

pub(super) fn move_leading_whitespace_to_previous_separator(
    chunks: Vec<SegmentedChunk>,
) -> Vec<SegmentedChunk> {
    let mut normalized: Vec<SegmentedChunk> = Vec::with_capacity(chunks.len());

    for mut chunk in chunks {
        let (leading_ws, text) = split_leading_whitespace(&chunk.text);
        if normalized.is_empty() || leading_ws.is_empty() || text.is_empty() {
            normalized.push(chunk);
            continue;
        }

        if let Some(previous) = normalized.last_mut() {
            previous.separator_after.push_str(&leading_ws);
        }
        chunk.text = text;
        if chunk.text.is_empty() {
            if let Some(previous) = normalized.last_mut() {
                previous.separator_after.push_str(&chunk.separator_after);
            }
            continue;
        }
        normalized.push(chunk);
    }

    normalized
}

fn try_merge_into_previous(merged: &mut [SegmentedChunk], chunk: &SegmentedChunk) -> bool {
    let Some(previous) = merged.last_mut() else {
        return false;
    };
    if !can_merge_left_binding_punctuation(previous, chunk) {
        return false;
    }

    previous.text.push_str(&chunk.text);
    previous.separator_after = chunk.separator_after.clone();
    true
}

fn can_merge_left_binding_punctuation(previous: &SegmentedChunk, chunk: &SegmentedChunk) -> bool {
    previous.skip_rewrite == chunk.skip_rewrite
        && previous.presentation == chunk.presentation
        && previous.separator_after.is_empty()
        && is_left_binding_punctuation_chunk(&chunk.text)
}

fn is_left_binding_punctuation_chunk(text: &str) -> bool {
    !text.is_empty() && text.chars().all(is_left_binding_punctuation)
}

fn is_left_binding_punctuation(ch: char) -> bool {
    LEFT_BINDING_PUNCTUATION.contains(&ch)
}

fn split_leading_whitespace(text: &str) -> (String, String) {
    let mut end = 0usize;
    for (index, ch) in text.char_indices() {
        if !ch.is_whitespace() {
            break;
        }
        end = index + ch.len_utf8();
    }
    (text[..end].to_string(), text[end..].to_string())
}
