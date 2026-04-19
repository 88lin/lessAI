use crate::{
    adapters::TextRegion,
    rewrite_unit::WritebackSlotRole,
    text_boundaries::split_text_and_trailing_separator,
    textual_template::models::TextTemplateRegion,
};

use super::inline_lines::{
    block_must_stay_locked, process_markdown_line, push_text_region, split_lines_with_endings,
};
use super::inline_scans::find_matching_bracket;
use super::inline_spans::find_markdown_link_end;
use super::syntax::markdown_line_prefix_len;

pub(super) fn build_regions(
    block_anchor: &str,
    block_text: &str,
    block_kind: &str,
    rewrite_headings: bool,
) -> Vec<TextTemplateRegion> {
    let regions = expand_locked_regions(parse_block_regions_for_kind(
        block_text,
        block_kind,
        rewrite_headings,
    ));

    regions
        .into_iter()
        .enumerate()
        .map(|(region_index, region)| build_region(block_anchor, region_index, region))
        .collect()
}

pub(super) fn parse_block_regions(text: &str, rewrite_headings: bool) -> Vec<TextRegion> {
    let block_kind = legacy_block_kind(text, rewrite_headings);
    parse_block_regions_for_kind(text, block_kind, rewrite_headings)
}

fn parse_block_regions_for_kind(
    text: &str,
    block_kind: &str,
    rewrite_headings: bool,
) -> Vec<TextRegion> {
    if text.is_empty() {
        return Vec::new();
    }
    if block_kind == "locked_block" || block_kind == "blank" {
        return vec![TextRegion {
            body: text.to_string(),
            skip_rewrite: true,
            presentation: None,
        }];
    }
    if block_kind == "heading" && !rewrite_headings {
        return vec![TextRegion {
            body: text.to_string(),
            skip_rewrite: true,
            presentation: None,
        }];
    }

    let lines = split_lines_with_endings(text);
    let mut out = Vec::new();
    for slice in lines {
        if slice.full.is_empty() {
            continue;
        }
        let ending = &slice.full[slice.line.len()..];
        process_markdown_line(&mut out, slice.line, ending);
    }
    out
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

fn expand_locked_regions(regions: Vec<TextRegion>) -> Vec<TextRegion> {
    let mut out = Vec::new();
    for region in regions {
        if !region.skip_rewrite {
            push_region_without_merging(&mut out, region);
            continue;
        }

        if let Some(expanded) = split_locked_link_region(&region.body) {
            for item in expanded {
                push_region_without_merging(&mut out, item);
            }
            continue;
        }

        push_region_without_merging(&mut out, region);
    }
    out
}

fn push_region_without_merging(regions: &mut Vec<TextRegion>, region: TextRegion) {
    if region.body.is_empty() {
        return;
    }
    regions.push(region);
}

fn split_locked_link_region(text: &str) -> Option<Vec<TextRegion>> {
    let (body, separator_after) = split_text_and_trailing_separator(text);
    if body.is_empty() {
        return None;
    }

    let mut out = Vec::new();
    let mut core = body.as_str();
    let prefix_len = markdown_line_prefix_len(core);
    if prefix_len > 0 && prefix_len < core.len() {
        let candidate = &core[prefix_len..];
        if candidate.starts_with('[') && !candidate.starts_with("![") {
            out.push(TextRegion {
                body: core[..prefix_len].to_string(),
                skip_rewrite: true,
                presentation: None,
            });
            core = candidate;
        }
    }

    if core.starts_with("![") || !core.starts_with('[') {
        return None;
    }
    if core.starts_with("[^")
        || core.starts_with("[@")
        || core.starts_with("[-@")
        || !has_parenthesized_link_target(core)?
        || find_markdown_link_end(core, 0)? != core.len()
    {
        return None;
    }

    let close = find_matching_bracket(core, 0)?;
    let label_start = 1usize;
    let label_end = close.saturating_sub(1);
    if label_end <= label_start {
        return None;
    }

    out.push(TextRegion {
        body: core[..label_start].to_string(),
        skip_rewrite: true,
        presentation: None,
    });

    let label = &core[label_start..label_end];
    for region in parse_block_regions_for_kind(label, "paragraph", true) {
        push_text_region(&mut out, region);
    }

    let mut closing = TextRegion {
        body: core[label_end..].to_string(),
        skip_rewrite: true,
        presentation: None,
    };
    closing.body.push_str(&separator_after);
    out.push(closing);
    Some(out)
}

fn has_parenthesized_link_target(text: &str) -> Option<bool> {
    let close = find_matching_bracket(text, 0)?;
    let bytes = text.as_bytes();
    let mut pos = close;
    while pos < bytes.len() && matches!(bytes[pos], b' ' | b'\t') {
        pos += 1;
    }
    Some(pos < bytes.len() && bytes[pos] == b'(')
}

fn legacy_block_kind(text: &str, rewrite_headings: bool) -> &'static str {
    let lines = split_lines_with_endings(text);
    if block_must_stay_locked(&lines, rewrite_headings) {
        return "locked_block";
    }
    "paragraph"
}
