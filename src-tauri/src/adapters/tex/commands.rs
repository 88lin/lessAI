use crate::{
    adapters::TextRegion,
    rewrite_unit::WritebackSlotRole,
    text_boundaries::split_text_and_trailing_separator,
    textual_template::models::TextTemplateRegion,
};

use super::scan::{
    consume_whitespace, find_closing_double_dollar, find_closing_single_dollar,
    find_command_span_end, find_inline_delimited_command_end, find_inline_verb_end,
    find_line_end, find_skip_environment_span, find_substring, is_escaped, parse_brace_group,
    parse_bracket_group,
};

const HEADING_COMMANDS: &[&str] = &[
    "section", "subsection", "subsubsection", "paragraph", "subparagraph", "chapter", "part",
    "title", "subtitle", "caption",
];
const TEXT_COMMANDS: &[&str] = &[
    "footnote", "emph", "textbf", "textit", "underline", "textrm", "textsf", "textsc",
];

pub(super) fn build_regions(block_anchor: &str, text: &str, rewrite_headings: bool) -> Vec<TextTemplateRegion> {
    parse_regions(text, rewrite_headings)
        .into_iter()
        .enumerate()
        .map(|(region_index, region)| build_region(block_anchor, region_index, region))
        .collect()
}

pub(super) fn parse_regions(text: &str, rewrite_headings: bool) -> Vec<TextRegion> {
    if text.is_empty() {
        return vec![TextRegion {
            body: String::new(),
            skip_rewrite: false,
            presentation: None,
        }];
    }

    let bytes = text.as_bytes();
    let mut regions: Vec<TextRegion> = Vec::new();
    let mut last = 0usize;
    let mut index = 0usize;

    while index < bytes.len() {
        let step = text[index..]
            .chars()
            .next()
            .map(char::len_utf8)
            .unwrap_or(1);

        if bytes[index] == b'%' && !is_escaped(text, index) {
            push_region(&mut regions, &text[last..index], false);
            let end = find_line_end(text, index);
            push_region(&mut regions, &text[index..end], true);
            index = end;
            last = end;
            continue;
        }

        if text[index..].starts_with("$$") && !is_escaped(text, index) {
            push_region(&mut regions, &text[last..index], false);
            let end = find_closing_double_dollar(text, index + 2).unwrap_or(text.len());
            push_region(&mut regions, &text[index..end], true);
            index = end;
            last = end;
            continue;
        }

        if bytes[index] == b'$'
            && !is_escaped(text, index)
            && !text[index..].starts_with("$$")
            && find_closing_single_dollar(text, index + 1).is_some()
        {
            push_region(&mut regions, &text[last..index], false);
            let end = find_closing_single_dollar(text, index + 1).unwrap_or(text.len());
            push_region(&mut regions, &text[index..end], true);
            index = end;
            last = end;
            continue;
        }

        if bytes[index] == b'\\' {
            if text[index..].starts_with("\\(") && !is_escaped(text, index) {
                if let Some(end) = find_substring(text, index + 2, "\\)") {
                    push_region(&mut regions, &text[last..index], false);
                    push_region(&mut regions, &text[index..end], true);
                    index = end;
                    last = end;
                    continue;
                }
            }

            if text[index..].starts_with("\\[") && !is_escaped(text, index) {
                if let Some(end) = find_substring(text, index + 2, "\\]") {
                    push_region(&mut regions, &text[last..index], false);
                    push_region(&mut regions, &text[index..end], true);
                    index = end;
                    last = end;
                    continue;
                }
            }

            if let Some((span_start, span_end)) = find_skip_environment_span(text, index) {
                push_region(&mut regions, &text[last..span_start], false);
                push_region(&mut regions, &text[span_start..span_end], true);
                index = span_end;
                last = span_end;
                continue;
            }

            if let Some(end) = find_inline_verb_end(text, index) {
                push_region(&mut regions, &text[last..index], false);
                push_region(&mut regions, &text[index..end], true);
                index = end;
                last = end;
                continue;
            }

            if let Some(end) = find_inline_delimited_command_end(text, index, "\\lstinline") {
                push_region(&mut regions, &text[last..index], false);
                push_region(&mut regions, &text[index..end], true);
                index = end;
                last = end;
                continue;
            }

            if let Some(end) = find_inline_delimited_command_end(text, index, "\\path") {
                push_region(&mut regions, &text[last..index], false);
                push_region(&mut regions, &text[index..end], true);
                index = end;
                last = end;
                continue;
            }

            if let Some((span_end, pieces)) = split_text_command_regions(text, index, rewrite_headings)
            {
                push_region(&mut regions, &text[last..index], false);
                for piece in pieces {
                    push_region(&mut regions, &piece.body, piece.skip_rewrite);
                }
                index = span_end;
                last = span_end;
                continue;
            }

            if let Some(end) = find_command_span_end(text, index) {
                push_region(&mut regions, &text[last..index], false);
                push_region(&mut regions, &text[index..end], true);
                index = end;
                last = end;
                continue;
            }
        }

        index += step;
    }

    push_region(&mut regions, &text[last..], false);
    if regions.is_empty() {
        return vec![TextRegion {
            body: text.to_string(),
            skip_rewrite: false,
            presentation: None,
        }];
    }
    regions
}

fn build_region(block_anchor: &str, region_index: usize, region: TextRegion) -> TextTemplateRegion {
    let (text, separator_after) = split_text_and_trailing_separator(&region.body);

    TextTemplateRegion {
        anchor: format!("{block_anchor}:r{region_index}"),
        text,
        editable: !region.skip_rewrite,
        role: if region.skip_rewrite {
            WritebackSlotRole::LockedText
        } else {
            WritebackSlotRole::EditableText
        },
        presentation: region.presentation,
        separator_after,
    }
}

fn push_region(regions: &mut Vec<TextRegion>, body: &str, skip_rewrite: bool) {
    if body.is_empty() {
        return;
    }
    if let Some(last) = regions.last_mut() {
        if last.skip_rewrite == skip_rewrite {
            last.body.push_str(body);
            return;
        }
    }
    regions.push(TextRegion {
        body: body.to_string(),
        skip_rewrite,
        presentation: None,
    });
}

fn split_text_command_regions(text: &str, index: usize, rewrite_headings: bool) -> Option<(usize, Vec<TextRegion>)> {
    let (name, mut pos) = parse_command_name(text, index)?;
    let name = name?;

    let is_heading_command = HEADING_COMMANDS.contains(&name);
    let allow_single_arg = TEXT_COMMANDS.contains(&name);
    if !is_heading_command && !allow_single_arg {
        return None;
    }

    let bytes = text.as_bytes();
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

    if bytes.get(pos) != Some(&b'{') {
        return None;
    }

    let group_end = parse_brace_group(text, pos)?;
    if group_end <= pos + 1 {
        return None;
    }
    let content_start = pos + 1;
    let content_end = group_end - 1;

    if is_heading_command && !rewrite_headings {
        return Some((
            group_end,
            vec![TextRegion {
                body: text[index..group_end].to_string(),
                skip_rewrite: true,
                presentation: None,
            }],
        ));
    }

    let mut out = vec![TextRegion {
        body: text[index..content_start].to_string(),
        skip_rewrite: true,
        presentation: None,
    }];
    out.extend(parse_regions(&text[content_start..content_end], rewrite_headings));
    out.push(TextRegion {
        body: text[content_end..group_end].to_string(),
        skip_rewrite: true,
        presentation: None,
    });

    Some((group_end, out))
}

fn parse_command_name(text: &str, index: usize) -> Option<(Option<&str>, usize)> {
    let bytes = text.as_bytes();
    if index >= bytes.len() || bytes[index] != b'\\' {
        return None;
    }

    let mut pos = index + 1;
    if pos >= bytes.len() {
        return None;
    }

    if bytes[pos].is_ascii_alphabetic() {
        let start = pos;
        while pos < bytes.len() && bytes[pos].is_ascii_alphabetic() {
            pos += 1;
        }
        let end = pos;
        if pos < bytes.len() && bytes[pos] == b'*' {
            pos += 1;
        }
        return Some((Some(&text[start..end]), pos));
    }

    pos += 1;
    Some((None, pos))
}
