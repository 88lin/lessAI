use crate::textual_template::{
    models::{TextTemplate, TextTemplateBlock},
};

use super::{blocks, commands};

pub(super) fn build_template(text: &str, rewrite_headings: bool) -> TextTemplate {
    let blocks = blocks::scan_blocks(text)
        .into_iter()
        .enumerate()
        .map(|(block_index, block)| build_block(block_index, block, rewrite_headings))
        .collect::<Vec<_>>();

    TextTemplate::new("tex", blocks)
}

fn build_block(
    block_index: usize,
    block: blocks::TexBlock,
    rewrite_headings: bool,
) -> TextTemplateBlock {
    let block_anchor = format!("tex:b{block_index}");

    TextTemplateBlock {
        anchor: block_anchor.clone(),
        kind: block.kind.to_string(),
        regions: commands::build_regions(&block_anchor, &block.text, rewrite_headings),
    }
}
