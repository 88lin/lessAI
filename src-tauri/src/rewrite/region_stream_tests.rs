use crate::adapters::tex::TexAdapter;
use crate::adapters::TextRegion;
use crate::documents::RegionSegmentationStrategy;
use crate::models::{ChunkPreset, DocumentFormat};

use super::*;

fn segment_text(text: &str, preset: ChunkPreset) -> Vec<SegmentedChunk> {
    segment_regions_with_strategy(
        TexAdapter::split_regions(text, false),
        preset,
        DocumentFormat::Tex,
        RegionSegmentationStrategy::FormatAware,
    )
}

#[test]
fn tex_par_command_splits_paragraph_chunks_without_emitting_separator_chunk() {
    let text = "第一句。\\par\n第二句。";
    let chunks = segment_text(text, ChunkPreset::Paragraph);

    let rebuilt = chunks
        .iter()
        .map(|chunk| format!("{}{}", chunk.text, chunk.separator_after))
        .collect::<String>();
    assert_eq!(rebuilt, text);

    let editable_chunks: Vec<&SegmentedChunk> = chunks.iter().filter(|c| !c.skip_rewrite).collect();
    assert_eq!(editable_chunks.len(), 2);
    assert_eq!(editable_chunks[0].text, "第一句。");
    assert_eq!(editable_chunks[1].text, "第二句。");
    assert!(!chunks.iter().any(|chunk| chunk.text == "\\par"));
}

#[test]
fn region_stream_keeps_inline_locked_regions_inside_same_sentence_chunk() {
    let regions = vec![
        super::segment::SegmentRegion::flow("前文 ", false, None),
        super::segment::SegmentRegion::flow("`let x = 1`", true, None),
        super::segment::SegmentRegion::flow(" 后文。下一句。", false, None),
    ];

    let chunks = super::segment::segment_region_stream::<super::segment::NoopBoundaryGuard>(
        regions,
        ChunkPreset::Sentence,
    );

    let rebuilt = chunks
        .iter()
        .map(|chunk| format!("{}{}", chunk.text, chunk.separator_after))
        .collect::<String>();
    assert_eq!(rebuilt, "前文 `let x = 1` 后文。下一句。");

    let editable_chunks: Vec<&SegmentedChunk> = chunks.iter().filter(|c| !c.skip_rewrite).collect();
    assert_eq!(editable_chunks.len(), 2);
    assert_eq!(editable_chunks[0].text, "前文 `let x = 1` 后文。");
    assert_eq!(editable_chunks[1].text, "下一句。");
}

#[test]
fn region_stream_outputs_isolated_skip_regions_as_standalone_chunks() {
    let regions = vec![
        super::segment::SegmentRegion::flow("前文。\n\n", false, None),
        super::segment::SegmentRegion::isolated("```rust\nfn main() {}\n```", true, None),
        super::segment::SegmentRegion::flow("\n\n后文。", false, None),
    ];

    let chunks = super::segment::segment_region_stream::<super::segment::NoopBoundaryGuard>(
        regions,
        ChunkPreset::Sentence,
    );

    assert!(chunks
        .iter()
        .any(|chunk| chunk.skip_rewrite && chunk.text.contains("fn main() {}")));

    let editable_chunks: Vec<&SegmentedChunk> = chunks.iter().filter(|c| !c.skip_rewrite).collect();
    assert_eq!(editable_chunks.len(), 2);
    assert_eq!(editable_chunks[0].text, "前文。");
    assert_eq!(editable_chunks[1].text, "后文。");
}

#[test]
fn region_stream_uses_separator_regions_as_boundaries_without_emitting_separator_chunks() {
    let regions = vec![
        super::segment::SegmentRegion::flow("第一句。", false, None),
        super::segment::SegmentRegion::separator("\\par\n"),
        super::segment::SegmentRegion::flow("第二句。", false, None),
    ];

    let chunks = super::segment::segment_region_stream::<super::segment::NoopBoundaryGuard>(
        regions,
        ChunkPreset::Paragraph,
    );

    let rebuilt = chunks
        .iter()
        .map(|chunk| format!("{}{}", chunk.text, chunk.separator_after))
        .collect::<String>();
    assert_eq!(rebuilt, "第一句。\\par\n第二句。");

    let editable_chunks: Vec<&SegmentedChunk> = chunks.iter().filter(|c| !c.skip_rewrite).collect();
    assert_eq!(editable_chunks.len(), 2);
    assert_eq!(editable_chunks[0].text, "第一句。");
    assert_eq!(editable_chunks[1].text, "第二句。");
    assert!(!chunks.iter().any(|chunk| chunk.text == "\\par"));
}

#[test]
fn preserve_boundaries_keeps_docx_like_locked_regions_isolated() {
    let regions = vec![
        TextRegion {
            body: "前文 ".to_string(),
            skip_rewrite: false,
            presentation: None,
        },
        TextRegion {
            body: "[图片]".to_string(),
            skip_rewrite: true,
            presentation: None,
        },
        TextRegion {
            body: " 后文。".to_string(),
            skip_rewrite: false,
            presentation: None,
        },
    ];

    let preserved = segment_regions_with_strategy(
        regions.clone(),
        ChunkPreset::Sentence,
        DocumentFormat::PlainText,
        RegionSegmentationStrategy::PreserveBoundaries,
    );
    let format_aware = segment_regions_with_strategy(
        regions,
        ChunkPreset::Sentence,
        DocumentFormat::PlainText,
        RegionSegmentationStrategy::FormatAware,
    );

    let preserved_text = preserved
        .iter()
        .map(|chunk| format!("{}{}", chunk.text, chunk.separator_after))
        .collect::<String>();
    let format_aware_text = format_aware
        .iter()
        .map(|chunk| format!("{}{}", chunk.text, chunk.separator_after))
        .collect::<String>();
    assert_eq!(preserved_text, "前文 [图片] 后文。");
    assert_eq!(format_aware_text, preserved_text);

    assert!(preserved
        .iter()
        .any(|chunk| chunk.skip_rewrite && chunk.text == "[图片]"));
    assert_eq!(
        preserved
            .iter()
            .filter(|chunk| !chunk.skip_rewrite)
            .map(|chunk| chunk.text.as_str())
            .collect::<Vec<_>>(),
        vec!["前文", " 后文。"]
    );

    let format_aware_editable = format_aware
        .iter()
        .filter(|chunk| !chunk.skip_rewrite)
        .map(|chunk| chunk.text.as_str())
        .collect::<Vec<_>>();
    assert_eq!(format_aware_editable, vec!["前文 [图片] 后文。"]);
    assert!(!format_aware
        .iter()
        .any(|chunk| chunk.skip_rewrite && chunk.text == "[图片]"));
}

#[test]
fn preserve_boundaries_attaches_standalone_colon_chunk_to_previous_chunk() {
    let regions = vec![
        TextRegion {
            body: "硬件部署".to_string(),
            skip_rewrite: false,
            presentation: None,
        },
        TextRegion {
            body: "：".to_string(),
            skip_rewrite: false,
            presentation: None,
        },
        TextRegion {
            body: "认知节点部署于 Dell。".to_string(),
            skip_rewrite: false,
            presentation: None,
        },
    ];

    let chunks = segment_regions_with_strategy(
        regions,
        ChunkPreset::Clause,
        DocumentFormat::PlainText,
        RegionSegmentationStrategy::PreserveBoundaries,
    );

    let editable = chunks
        .iter()
        .filter(|chunk| !chunk.skip_rewrite)
        .map(|chunk| chunk.text.as_str())
        .collect::<Vec<_>>();
    assert_eq!(editable, vec!["硬件部署：", "认知节点部署于 Dell。"]);
}

#[test]
fn preserve_boundaries_keeps_standalone_colon_chunk_when_presentation_differs() {
    let colon_presentation = Some(crate::models::ChunkPresentation {
        bold: true,
        italic: false,
        underline: false,
        href: None,
        protect_kind: None,
        writeback_key: Some("r:bold".to_string()),
    });
    let regions = vec![
        TextRegion {
            body: "硬件部署".to_string(),
            skip_rewrite: false,
            presentation: None,
        },
        TextRegion {
            body: "：".to_string(),
            skip_rewrite: false,
            presentation: colon_presentation,
        },
        TextRegion {
            body: "认知节点部署于 Dell。".to_string(),
            skip_rewrite: false,
            presentation: None,
        },
    ];

    let chunks = segment_regions_with_strategy(
        regions,
        ChunkPreset::Clause,
        DocumentFormat::PlainText,
        RegionSegmentationStrategy::PreserveBoundaries,
    );

    let editable = chunks
        .iter()
        .filter(|chunk| !chunk.skip_rewrite)
        .map(|chunk| chunk.text.as_str())
        .collect::<Vec<_>>();
    assert_eq!(editable, vec!["硬件部署", "：", "认知节点部署于 Dell。"]);
}

#[test]
fn preserve_boundaries_keeps_whitespace_only_editable_regions_as_chunks() {
    let underline = Some(crate::models::ChunkPresentation {
        bold: false,
        italic: false,
        underline: true,
        href: None,
        protect_kind: None,
        writeback_key: Some("r:underline".to_string()),
    });
    let regions = vec![
        TextRegion {
            body: "作品编号：".to_string(),
            skip_rewrite: false,
            presentation: None,
        },
        TextRegion {
            body: "　　　\n\n".to_string(),
            skip_rewrite: false,
            presentation: underline.clone(),
        },
    ];

    let chunks = segment_regions_with_strategy(
        regions,
        ChunkPreset::Paragraph,
        DocumentFormat::PlainText,
        RegionSegmentationStrategy::PreserveBoundaries,
    );

    assert_eq!(chunks.len(), 2);
    assert_eq!(chunks[0].text, "作品编号：");
    assert_eq!(chunks[1].text, "　　　");
    assert_eq!(chunks[1].separator_after, "\n\n");
    assert_eq!(chunks[1].presentation, underline);
    assert!(!chunks[1].skip_rewrite);
}
