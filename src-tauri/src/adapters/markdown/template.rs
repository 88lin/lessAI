use crate::textual_template::{models::TextTemplateBlock, TextTemplate};

use super::{block_support::MarkdownBlock, blocks, inline};

pub(super) fn build_template(text: &str, rewrite_headings: bool) -> TextTemplate {
    let blocks = blocks::scan_blocks(text)
        .into_iter()
        .enumerate()
        .map(|(block_index, block)| build_block(block_index, block, rewrite_headings))
        .collect::<Vec<_>>();

    TextTemplate::new("markdown", blocks)
}

fn build_block(block_index: usize, block: MarkdownBlock, rewrite_headings: bool) -> TextTemplateBlock {
    let block_anchor = format!("md:b{block_index}");

    TextTemplateBlock {
        anchor: block_anchor.clone(),
        kind: block.kind.to_string(),
        regions: inline::build_regions(&block_anchor, &block.text, block.kind, rewrite_headings),
    }
}
