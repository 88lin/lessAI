use crate::adapters::TextRegion;
use crate::documents::RegionSegmentationStrategy;
use crate::models::{ChunkPreset, DocumentFormat};

use super::guards::{NoopBoundaryGuard, TexBraceBoundaryGuard};
use super::postprocess::{
    merge_left_binding_punctuation_chunks, move_leading_whitespace_to_previous_separator,
};
use super::stream::{segment_region_stream, SegmentRegion};
use super::SegmentedChunk;

const PRESERVED_BLOCK_SEPARATOR: &str = "\n\n";

fn segment_preserved_regions(regions: Vec<TextRegion>, preset: ChunkPreset) -> Vec<SegmentedChunk> {
    if preset == ChunkPreset::Paragraph {
        return regions
            .into_iter()
            .filter_map(preserved_paragraph_chunk)
            .collect();
    }
    let stream = regions
        .into_iter()
        .filter(|region| !region.body.is_empty())
        .map(|region| {
            SegmentRegion::isolated(region.body, region.skip_rewrite, region.presentation)
        })
        .collect::<Vec<_>>();
    segment_region_stream::<NoopBoundaryGuard>(stream, preset)
}

fn preserved_paragraph_chunk(region: TextRegion) -> Option<SegmentedChunk> {
    let (text, separator_after) = split_preserved_region_text(&region.body);
    if text.is_empty() && separator_after.is_empty() {
        return None;
    }
    Some(SegmentedChunk {
        skip_rewrite: region.skip_rewrite,
        text,
        separator_after,
        presentation: region.presentation,
    })
}

fn split_preserved_region_text(body: &str) -> (String, String) {
    match body.strip_suffix(PRESERVED_BLOCK_SEPARATOR) {
        Some(text) => (text.to_string(), PRESERVED_BLOCK_SEPARATOR.to_string()),
        None => (body.to_string(), String::new()),
    }
}

pub fn segment_regions_with_strategy(
    regions: Vec<TextRegion>,
    preset: ChunkPreset,
    format: DocumentFormat,
    strategy: RegionSegmentationStrategy,
) -> Vec<SegmentedChunk> {
    match strategy {
        RegionSegmentationStrategy::PreserveBoundaries => {
            merge_left_binding_punctuation_chunks(segment_preserved_regions(regions, preset))
        }
        RegionSegmentationStrategy::FormatAware => move_leading_whitespace_to_previous_separator(
            merge_left_binding_punctuation_chunks(segment_text_regions(regions, preset, format)),
        ),
    }
}

fn segment_text_regions(
    regions: Vec<TextRegion>,
    preset: ChunkPreset,
    format: DocumentFormat,
) -> Vec<SegmentedChunk> {
    let stream = match format {
        DocumentFormat::PlainText => regions
            .into_iter()
            .filter(|region| !region.body.is_empty())
            .map(|region| {
                SegmentRegion::flow(region.body, region.skip_rewrite, region.presentation)
            })
            .collect::<Vec<_>>(),
        DocumentFormat::Markdown => super::markdown::build_markdown_stream(regions),
        DocumentFormat::Tex => super::tex::build_tex_stream(regions),
    };

    match format {
        DocumentFormat::Tex => segment_region_stream::<TexBraceBoundaryGuard>(stream, preset),
        DocumentFormat::PlainText | DocumentFormat::Markdown => {
            segment_region_stream::<NoopBoundaryGuard>(stream, preset)
        }
    }
}
