use super::DocxAdapter;
use crate::{
    models::SegmentationPreset,
    rewrite_unit::build_rewrite_units,
    test_support::{
        build_minimal_docx, build_report_template_fixture_docx, load_repo_docx_fixture_or,
    },
};

fn editable_unit_texts(bytes: &[u8], preset: SegmentationPreset) -> Vec<String> {
    let slots = DocxAdapter::extract_writeback_slots(bytes, false).expect("extract slots");
    build_rewrite_units(&slots, preset)
        .into_iter()
        .filter(|unit| {
            unit.slot_ids.iter().any(|slot_id| {
                slots
                    .iter()
                    .any(|slot| slot.id == *slot_id && slot.editable)
            })
        })
        .map(|unit| unit.display_text)
        .collect()
}

fn load_repo_docx_fixture(file_name: &str) -> Vec<u8> {
    load_repo_docx_fixture_or(file_name, build_report_template_fixture_docx)
}

#[test]
fn clause_preset_splits_single_docx_region_on_comma_boundaries() {
    let xml = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
  <w:body>
    <w:p>
      <w:r><w:t>甲，乙，丙</w:t></w:r>
    </w:p>
  </w:body>
</w:document>"#;
    let bytes = build_minimal_docx(xml);

    let units = editable_unit_texts(&bytes, SegmentationPreset::Clause);

    assert_eq!(units, vec!["甲，乙，".to_string(), "丙".to_string()]);
}

#[test]
fn sentence_preset_keeps_single_docx_region_together_without_sentence_boundary() {
    let xml = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
  <w:body>
    <w:p>
      <w:r><w:t>甲，乙，丙</w:t></w:r>
    </w:p>
  </w:body>
</w:document>"#;
    let bytes = build_minimal_docx(xml);

    let units = editable_unit_texts(&bytes, SegmentationPreset::Sentence);

    assert_eq!(units, vec!["甲，乙，丙".to_string()]);
}

#[test]
fn clause_preset_splits_report_template_requirement_sections() {
    let bytes = load_repo_docx_fixture("04-3 作品报告（大数据应用赛，2025版）模板.docx");

    let units = editable_unit_texts(&bytes, SegmentationPreset::Clause);

    assert!(
        !units.iter().any(|unit| {
            unit.contains("作品功能需求主要包括") && unit.contains("系统性能需求包括")
        }),
        "clause units should not keep multiple requirement sections together: {units:#?}"
    );
    assert!(
        !units.iter().any(|unit| {
            unit.contains("本作品所使用的数据集主要由公开数据和自采数据两部分构成")
                && unit.contains("数据类型涵盖结构化表格数据")
        }),
        "clause units should not keep multiple dataset sections together: {units:#?}"
    );
}
