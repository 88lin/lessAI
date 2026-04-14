use super::model::{LockedDisplayMode, WritebackBlockTemplate, WritebackRegionTemplate};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DisplayBlockKind {
    Paragraph { block_index: usize },
    LockedBlock { block_index: usize },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DisplayBlockRef {
    pub kind: DisplayBlockKind,
    pub region_indices: Vec<usize>,
}

pub(crate) fn build_display_blocks(blocks: &[WritebackBlockTemplate]) -> Vec<DisplayBlockRef> {
    let mut out = Vec::new();
    for (block_index, block) in blocks.iter().enumerate() {
        match block {
            WritebackBlockTemplate::Paragraph(paragraph) => {
                out.extend(build_paragraph_display_blocks(
                    block_index,
                    &paragraph.regions,
                ));
            }
            WritebackBlockTemplate::Locked(_) => out.push(DisplayBlockRef {
                kind: DisplayBlockKind::LockedBlock { block_index },
                region_indices: Vec::new(),
            }),
        }
    }
    out
}

fn build_paragraph_display_blocks(
    block_index: usize,
    regions: &[WritebackRegionTemplate],
) -> Vec<DisplayBlockRef> {
    if regions.is_empty() {
        return vec![DisplayBlockRef {
            kind: DisplayBlockKind::Paragraph { block_index },
            region_indices: Vec::new(),
        }];
    }

    let mut inline = Vec::new();
    let mut trailing = Vec::new();
    for (region_index, region) in regions.iter().enumerate() {
        match region {
            WritebackRegionTemplate::Locked(region)
                if region.display_mode == LockedDisplayMode::AfterParagraph =>
            {
                trailing.push(DisplayBlockRef {
                    kind: DisplayBlockKind::Paragraph { block_index },
                    region_indices: vec![region_index],
                });
            }
            _ => inline.push(region_index),
        }
    }

    let mut out = Vec::new();
    if !inline.is_empty() {
        out.push(DisplayBlockRef {
            kind: DisplayBlockKind::Paragraph { block_index },
            region_indices: inline,
        });
    }
    out.extend(trailing);
    out
}
