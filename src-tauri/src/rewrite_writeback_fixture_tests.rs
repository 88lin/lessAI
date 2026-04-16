use std::{collections::HashMap, fs, path::PathBuf};

use chrono::Utc;

use crate::{
    adapters::docx::DocxAdapter,
    document_snapshot::capture_document_snapshot,
    documents::{RegionSegmentationStrategy, WritebackMode},
    models::{ChunkPreset, ChunkStatus, ChunkTask, DocumentSession, RunningState},
    rewrite,
    rewrite_projection::build_merged_regions,
    test_support::{cleanup_dir, write_temp_file},
};

fn load_report_template_fixture() -> Vec<u8> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("testdoc")
        .join("04-3 作品报告（大数据应用赛，2025版）模板.docx");
    fs::read(path).expect("read report template fixture")
}

fn build_report_template_session(path: &std::path::Path, bytes: &[u8]) -> DocumentSession {
    let now = Utc::now();
    let regions = DocxAdapter::extract_regions(bytes, false).expect("extract regions");
    let source_text = regions
        .iter()
        .map(|region| region.body.as_str())
        .collect::<String>();
    let chunks = rewrite::segment_regions_with_strategy(
        regions,
        ChunkPreset::Paragraph,
        crate::models::DocumentFormat::PlainText,
        RegionSegmentationStrategy::PreserveBoundaries,
    )
    .into_iter()
    .enumerate()
    .map(|(index, chunk)| ChunkTask {
        index,
        source_text: chunk.text,
        separator_after: chunk.separator_after,
        skip_rewrite: chunk.skip_rewrite,
        presentation: chunk.presentation,
        status: if chunk.skip_rewrite {
            ChunkStatus::Done
        } else {
            ChunkStatus::Idle
        },
        error_message: None,
    })
    .collect::<Vec<_>>();

    DocumentSession {
        id: "report-template-session".to_string(),
        title: "模板".to_string(),
        document_path: path.to_string_lossy().to_string(),
        source_text: source_text.clone(),
        source_snapshot: Some(capture_document_snapshot(path).expect("capture snapshot")),
        normalized_text: source_text,
        write_back_supported: true,
        write_back_block_reason: None,
        plain_text_editor_safe: true,
        plain_text_editor_block_reason: None,
        chunk_preset: Some(ChunkPreset::Paragraph),
        rewrite_headings: Some(false),
        chunks,
        suggestions: Vec::new(),
        next_suggestion_sequence: 1,
        status: RunningState::Idle,
        created_at: now,
        updated_at: now,
    }
}

fn find_chunk_index(session: &DocumentSession, source_text: &str) -> usize {
    session
        .chunks
        .iter()
        .find(|chunk| chunk.source_text == source_text)
        .map(|chunk| chunk.index)
        .expect("find chunk")
}

fn find_second_chunk_index(session: &DocumentSession, source_text: &str) -> usize {
    session
        .chunks
        .iter()
        .filter(|chunk| chunk.source_text == source_text)
        .nth(1)
        .map(|chunk| chunk.index)
        .expect("find second chunk")
}

fn assert_session_regions_roundtrip(
    session: &DocumentSession,
    expected_regions: &[crate::adapters::TextRegion],
) {
    let merged = build_merged_regions(session, None);
    let merged_text = merged
        .iter()
        .map(|region| region.body.as_str())
        .collect::<String>();
    let expected_text = expected_regions
        .iter()
        .map(|region| region.body.as_str())
        .collect::<String>();
    assert_eq!(
        merged_text, expected_text,
        "merged text drifted from imported regions"
    );
    assert_eq!(
        merged.len(),
        expected_regions.len(),
        "region count mismatch: merged={} expected={} missing_boundaries={:?}",
        merged.len(),
        expected_regions.len(),
        missing_boundaries(&merged, expected_regions),
    );
    let mut skip_mismatches = Vec::new();
    for (index, (actual, expected)) in merged.iter().zip(expected_regions.iter()).enumerate() {
        assert_eq!(
            actual.body, expected.body,
            "region body mismatch at index {index}: actual={actual:?} expected={expected:?}"
        );
        if actual.skip_rewrite != expected.skip_rewrite {
            skip_mismatches.push(format!(
                "index={index} actual={actual:?} expected={expected:?} actual_window={:?} expected_window={:?}",
                region_window(&merged, index),
                region_window(expected_regions, index),
            ));
        }
        assert_eq!(
            actual.presentation, expected.presentation,
            "region presentation mismatch at index {index}: actual={actual:?} expected={expected:?}"
        );
    }
    assert!(
        skip_mismatches.is_empty(),
        "region skip mismatches: {skip_mismatches:#?}"
    );
}

fn region_window(regions: &[crate::adapters::TextRegion], index: usize) -> Vec<String> {
    let start = index.saturating_sub(1);
    let end = (index + 2).min(regions.len());
    regions[start..end]
        .iter()
        .map(|region| format!("{region:?}"))
        .collect()
}

fn missing_boundaries(
    merged: &[crate::adapters::TextRegion],
    expected_regions: &[crate::adapters::TextRegion],
) -> Vec<String> {
    let merged_boundaries = boundary_positions(merged);
    let expected_boundaries = boundary_positions(expected_regions);
    expected_boundaries
        .into_iter()
        .filter(|boundary| !merged_boundaries.contains(boundary))
        .map(|boundary| boundary_context(expected_regions, boundary))
        .collect()
}

fn boundary_positions(regions: &[crate::adapters::TextRegion]) -> Vec<usize> {
    let mut total = 0usize;
    regions
        .iter()
        .map(|region| {
            total += region.body.chars().count();
            total
        })
        .collect()
}

fn boundary_context(regions: &[crate::adapters::TextRegion], boundary: usize) -> String {
    let text = regions
        .iter()
        .map(|region| region.body.as_str())
        .collect::<String>();
    let chars = text.chars().collect::<Vec<_>>();
    let start = boundary.saturating_sub(12);
    let end = (boundary + 12).min(chars.len());
    chars[start..end].iter().collect()
}

#[test]
fn execute_session_writeback_validates_unmodified_report_template_session() {
    let bytes = load_report_template_fixture();
    let (root, target) = write_temp_file("report-template-validate", "docx", &bytes);
    let expected_regions = DocxAdapter::extract_regions(&bytes, false).expect("extract regions");
    let session = build_report_template_session(&target, &bytes);
    let reference_index = find_second_chunk_index(&session, "参考文献");
    assert_eq!(session.chunks[reference_index].separator_after, "");
    assert!(session.chunks[reference_index + 1]
        .source_text
        .starts_with('\n'));

    assert_session_regions_roundtrip(&session, &expected_regions);

    super::execute_session_writeback(&session, WritebackMode::Validate)
        .expect("expected unmodified report template session to validate");

    cleanup_dir(&root);
}

#[test]
fn validate_candidate_batch_writeback_accepts_partial_adjacent_styled_regions_in_report_template() {
    let bytes = load_report_template_fixture();
    let (root, target) = write_temp_file("report-template-partial-batch", "docx", &bytes);
    let session = build_report_template_session(&target, &bytes);
    let first = "【填写说明：从工程实现的角度，";
    let second =
        "详细说明第3章所提技术方案的具体实现过程，内容包括但不限于软件设计与实现、用户界面、数据来源、数据训练、改进过程（尤其说明技术上的增量部分）以及系统部署方法";
    let third = "等，以及实施过程中遇到的困难和相应的解决方法】";
    let first_index = find_chunk_index(&session, first);
    let second_index = find_chunk_index(&session, second);
    let third_index = find_chunk_index(&session, third);

    assert_eq!(second_index, first_index + 1);
    assert_eq!(third_index, second_index + 1);

    let overrides = HashMap::from([
        (first_index, "【填写说明：从工程实现层面，".to_string()),
        (
            second_index,
            "详细说明第3章技术方案的工程化落地过程，内容包括但不限于软件设计与实现、用户界面、数据来源、数据训练、改进过程（尤其说明技术上的增量部分）以及系统部署方法"
                .to_string(),
        ),
    ]);

    super::validate_candidate_batch_writeback(&session, &overrides)
        .expect("expected partial adjacent styled regions in template to validate");

    cleanup_dir(&root);
}
