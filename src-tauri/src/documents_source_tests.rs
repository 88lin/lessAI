use crate::{
    adapters::{tex::TexAdapter, TextRegion},
    documents::{load_document_source, source::writeback_slots_from_regions},
    models::{SegmentationPreset, TextPresentation},
    rewrite_unit::build_rewrite_units,
    test_support::{build_minimal_docx, cleanup_dir, write_temp_file},
};

#[test]
fn writeback_slots_split_preserved_block_separator_from_region_body() {
    let slots = writeback_slots_from_regions(&[TextRegion {
        body: "第一段\n\n".to_string(),
        skip_rewrite: false,
        presentation: None,
    }]);

    assert_eq!(slots.len(), 1);
    assert_eq!(slots[0].text, "第一段");
    assert_eq!(slots[0].separator_after, "\n\n");
    assert!(slots[0].editable);
}

#[test]
fn writeback_slots_lock_whitespace_only_regions_even_when_region_is_editable() {
    let underline = Some(TextPresentation {
        bold: false,
        italic: false,
        underline: true,
        href: None,
        protect_kind: None,
        writeback_key: Some("r:underline".to_string()),
    });
    let slots = writeback_slots_from_regions(&[TextRegion {
        body: "　　　\n\n".to_string(),
        skip_rewrite: false,
        presentation: underline.clone(),
    }]);

    assert_eq!(slots.len(), 1);
    assert_eq!(slots[0].text, "　　　");
    assert_eq!(slots[0].separator_after, "\n\n");
    assert!(!slots[0].editable);
    assert_eq!(slots[0].presentation, underline);
}

#[test]
fn writeback_slots_preserve_paragraph_boundaries_for_rewrite_units() {
    let slots = writeback_slots_from_regions(&[
        TextRegion {
            body: "第一段\n\n".to_string(),
            skip_rewrite: false,
            presentation: None,
        },
        TextRegion {
            body: "第二段".to_string(),
            skip_rewrite: false,
            presentation: None,
        },
    ]);

    let units = build_rewrite_units(&slots, SegmentationPreset::Paragraph);

    assert_eq!(units.len(), 2);
    assert_eq!(units[0].display_text, "第一段\n\n");
    assert_eq!(units[1].display_text, "第二段");
}

#[test]
fn tex_single_source_newline_does_not_split_paragraph_units() {
    let regions = TexAdapter::split_regions("第一句。\n第二句。", false);
    let slots = writeback_slots_from_regions(&regions);

    let units = build_rewrite_units(&slots, SegmentationPreset::Paragraph);

    assert_eq!(units.len(), 1);
    assert_eq!(units[0].display_text, "第一句。\n第二句。");
}

#[test]
fn tex_blank_line_boundaries_split_heading_and_paragraph_units() {
    let text = "\\section{标题}\n\n第一段第一行。\n第一段第二行。\n\n第二段。";
    let regions = TexAdapter::split_regions(text, false);
    let slots = writeback_slots_from_regions(&regions);

    let units = build_rewrite_units(&slots, SegmentationPreset::Paragraph);

    assert_eq!(units.len(), 3);
    assert_eq!(units[0].display_text, "\\section{标题}\n\n");
    assert_eq!(units[1].display_text, "第一段第一行。\n第一段第二行。\n\n");
    assert_eq!(units[2].display_text, "第二段。");
}

#[test]
fn crlf_blank_line_boundaries_split_paragraph_units() {
    let slots = writeback_slots_from_regions(&[TextRegion {
        body: "第一段\r\n\r\n第二段".to_string(),
        skip_rewrite: false,
        presentation: None,
    }]);

    let units = build_rewrite_units(&slots, SegmentationPreset::Paragraph);

    assert_eq!(units.len(), 2);
    assert_eq!(units[0].display_text, "第一段\r\n\r\n");
    assert_eq!(units[1].display_text, "第二段");
}

#[test]
fn load_docx_source_marks_page_break_placeholder_as_inline_object_slot() {
    let xml = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
  <w:body>
    <w:p>
      <w:r><w:t>上文</w:t></w:r>
      <w:r><w:br w:type="page"/></w:r>
      <w:r><w:t>下文</w:t></w:r>
    </w:p>
  </w:body>
</w:document>"#;
    let bytes = build_minimal_docx(xml);
    let (root, path) = write_temp_file("docx-slot-page-break", "docx", &bytes);

    let loaded = load_document_source(&path, false).expect("load docx source");
    let slot = loaded
        .writeback_slots
        .iter()
        .find(|slot| slot.text == "[分页符]")
        .expect("page break slot");

    assert_eq!(
        slot.role,
        crate::rewrite_unit::WritebackSlotRole::InlineObject
    );
    assert!(!slot.editable);

    cleanup_dir(&root);
}
