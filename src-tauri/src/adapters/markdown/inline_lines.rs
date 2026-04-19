use crate::adapters::TextRegion;

use super::inline_emphasis::{find_matching_emphasis, parse_emphasis_delimiter_run};
use super::inline_spans::find_markdown_protected_spans;
use super::syntax::{
    detect_fence_marker, is_atx_heading_line, is_html_like_line, is_horizontal_rule_line,
    is_markdown_table_delimiter, is_math_block_delimiter_line, is_reference_definition_line,
    is_setext_underline_line, is_yaml_front_matter_close, is_yaml_front_matter_open,
    markdown_line_prefix_len,
};

#[derive(Debug, Clone, Copy)]
pub(super) struct LineSlice<'a> {
    pub line: &'a str,
    pub full: &'a str,
}

const MAX_FRONT_MATTER_LINES: usize = 200;

pub(super) fn split_lines_with_endings(text: &str) -> Vec<LineSlice<'_>> {
    let bytes = text.as_bytes();
    let mut lines: Vec<LineSlice<'_>> = Vec::new();
    let mut start = 0usize;
    let mut index = 0usize;

    while index < bytes.len() {
        match bytes[index] {
            b'\n' => {
                lines.push(LineSlice {
                    line: &text[start..index],
                    full: &text[start..index + 1],
                });
                index += 1;
                start = index;
            }
            b'\r' => {
                let end = if index + 1 < bytes.len() && bytes[index + 1] == b'\n' {
                    index + 2
                } else {
                    index + 1
                };
                lines.push(LineSlice {
                    line: &text[start..index],
                    full: &text[start..end],
                });
                index = end;
                start = index;
            }
            _ => index += 1,
        }
    }

    if start < bytes.len() {
        lines.push(LineSlice {
            line: &text[start..bytes.len()],
            full: &text[start..bytes.len()],
        });
    } else if text.is_empty() {
        lines.push(LineSlice { line: "", full: "" });
    }

    lines
}

pub(super) fn block_must_stay_locked(
    lines: &[LineSlice<'_>],
    rewrite_headings: bool,
) -> bool {
    let Some(first_non_blank) = lines.iter().position(|slice| !slice.line.trim().is_empty()) else {
        return true;
    };

    if let Some((start, end)) = find_yaml_front_matter_range(lines) {
        if start == first_non_blank
            && lines
                .iter()
                .skip(end + 1)
                .all(|slice| slice.line.trim().is_empty())
        {
            return true;
        }
    }

    let first_line = lines[first_non_blank].line;
    let trailing_blank_only = lines
        .iter()
        .skip(first_non_blank + 1)
        .all(|slice| slice.line.trim().is_empty());

    if detect_fence_marker(first_line).is_some() || is_math_block_delimiter_line(first_line) {
        return true;
    }
    if first_non_blank + 1 < lines.len()
        && !first_line.trim().is_empty()
        && first_line.contains('|')
        && is_markdown_table_delimiter(lines[first_non_blank + 1].line)
    {
        return true;
    }
    if !rewrite_headings
        && (is_atx_heading_line(first_line)
            || (first_non_blank + 1 < lines.len()
                && is_setext_underline_line(lines[first_non_blank + 1].line)))
    {
        return true;
    }
    trailing_blank_only
        && (is_reference_definition_line(first_line)
            || is_html_like_line(first_line)
            || is_horizontal_rule_line(first_line))
}

pub(super) fn process_markdown_line(out: &mut Vec<TextRegion>, line: &str, ending: &str) {
    let prefix_len = markdown_line_prefix_len(line);
    let (prefix, core) = if prefix_len > 0 && prefix_len <= line.len() {
        (&line[..prefix_len], &line[prefix_len..])
    } else {
        ("", line)
    };

    if !prefix.is_empty() {
        push_text_region(
            out,
            TextRegion {
                body: prefix.to_string(),
                skip_rewrite: true,
                presentation: None,
            },
        );
    }

    let spans = find_markdown_protected_spans(core);
    if spans.is_empty() {
        push_rewriteable_markdown_text(out, core);
        append_line_ending(out, ending);
        return;
    }

    let mut cursor = 0usize;
    for (start, end) in spans {
        if start > cursor {
            push_rewriteable_markdown_text(out, &core[cursor..start]);
        }
        push_text_region(
            out,
            TextRegion {
                body: core[start..end].to_string(),
                skip_rewrite: true,
                presentation: None,
            },
        );
        cursor = end;
    }
    if cursor < core.len() {
        push_rewriteable_markdown_text(out, &core[cursor..]);
    }

    append_line_ending(out, ending);
}

pub(super) fn push_text_region(regions: &mut Vec<TextRegion>, region: TextRegion) {
    if region.body.is_empty() {
        return;
    }

    if let Some(last) = regions.last_mut() {
        if last.skip_rewrite == region.skip_rewrite {
            last.body.push_str(&region.body);
            return;
        }
    }

    regions.push(region);
}

fn find_yaml_front_matter_range(lines: &[LineSlice<'_>]) -> Option<(usize, usize)> {
    let mut index = 0usize;
    while index < lines.len() && lines[index].line.trim().is_empty() {
        index += 1;
    }
    if index >= lines.len() || !is_yaml_front_matter_open(lines[index].line) {
        return None;
    }

    let start = index;
    let end_limit = (start + MAX_FRONT_MATTER_LINES).min(lines.len().saturating_sub(1));
    index += 1;
    while index <= end_limit {
        if is_yaml_front_matter_close(lines[index].line) {
            return Some((start, index));
        }
        index += 1;
    }
    None
}

fn append_line_ending(out: &mut Vec<TextRegion>, ending: &str) {
    if ending.is_empty() {
        return;
    }
    if let Some(last) = out.last_mut() {
        last.body.push_str(ending);
    } else {
        out.push(TextRegion {
            body: ending.to_string(),
            skip_rewrite: true,
            presentation: None,
        });
    }
}

fn push_rewriteable_markdown_text(out: &mut Vec<TextRegion>, text: &str) {
    if text.is_empty() {
        return;
    }

    let bytes = text.as_bytes();
    let mut cursor = 0usize;
    let mut index = 0usize;
    while index < bytes.len() {
        let Some(run) = parse_emphasis_delimiter_run(text, index) else {
            index += 1;
            continue;
        };
        if !run.can_open {
            index = run.end;
            continue;
        }
        let Some((open_len, close_start, close_len)) = find_matching_emphasis(text, run) else {
            index = run.end;
            continue;
        };

        if run.start > cursor {
            push_editable_markdown_text(out, &text[cursor..run.start]);
        }

        push_locked_markdown_text(out, &text[run.start..run.start + open_len]);

        let inner_start = run.start + open_len;
        let inner_end = close_start;
        if inner_end > inner_start {
            push_rewriteable_markdown_text(out, &text[inner_start..inner_end]);
        }

        push_locked_markdown_text(out, &text[close_start..close_start + close_len]);

        cursor = close_start + close_len;
        index = cursor;
    }

    if cursor < text.len() {
        push_editable_markdown_text(out, &text[cursor..]);
    }
}

fn push_editable_markdown_text(out: &mut Vec<TextRegion>, text: &str) {
    push_text_region(
        out,
        TextRegion {
            body: text.to_string(),
            skip_rewrite: false,
            presentation: None,
        },
    );
}

fn push_locked_markdown_text(out: &mut Vec<TextRegion>, text: &str) {
    push_text_region(
        out,
        TextRegion {
            body: text.to_string(),
            skip_rewrite: true,
            presentation: None,
        },
    );
}
