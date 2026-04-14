use std::{fs, io::Write, path::PathBuf};

use super::DocxAdapter;
use zip::{write::FileOptions, ZipWriter};

fn build_docx_entries(entries: &[(&str, &str)]) -> Vec<u8> {
    let mut out = Vec::new();
    let cursor = std::io::Cursor::new(&mut out);
    let mut zip = ZipWriter::new(cursor);
    let options = FileOptions::<()>::default();

    for (name, contents) in entries {
        zip.start_file(*name, options).expect("start file");
        zip.write_all(contents.as_bytes()).expect("write xml");
    }
    zip.finish().expect("finish zip");
    out
}

fn assert_locked(regions: &[crate::adapters::TextRegion], needle: &str) {
    assert!(
        regions
            .iter()
            .any(|region| region.skip_rewrite && region.body.contains(needle)),
        "expected locked region containing `{needle}`, got:\n{}",
        regions
            .iter()
            .map(|region| region.body.as_str())
            .collect::<String>()
    );
}

fn assert_editable(regions: &[crate::adapters::TextRegion], needle: &str) {
    assert!(
        regions
            .iter()
            .any(|region| !region.skip_rewrite && region.body.contains(needle)),
        "expected editable region containing `{needle}`, got:\n{}",
        regions
            .iter()
            .map(|region| region.body.as_str())
            .collect::<String>()
    );
}

fn production_docx_sources() -> Vec<PathBuf> {
    [
        "src/adapters/docx/display.rs",
        "src/adapters/docx/model.rs",
        "src/adapters/docx/numbering.rs",
        "src/adapters/docx/placeholders.rs",
        "src/adapters/docx/simple.rs",
        "src/adapters/docx/specials.rs",
        "src/adapters/docx/styles.rs",
    ]
    .into_iter()
    .map(|relative| PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(relative))
    .collect()
}

fn banned_semantic_fragments() -> &'static [&'static str] {
    &[
        "Heading1",
        "heading 1",
        "table of contents",
        "\"toc\"",
        "\"fill-line\"",
        "[目录]",
        "封面",
        "摘要",
        "附录",
        "caption",
        "subtitle",
    ]
}

#[test]
fn does_not_treat_heading_like_style_ids_as_headings_without_structural_definition() {
    let document_xml = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
  <w:body>
    <w:p>
      <w:pPr><w:pStyle w:val="Heading1"/></w:pPr>
      <w:r><w:t>标题</w:t></w:r>
    </w:p>
    <w:p><w:r><w:t>正文</w:t></w:r></w:p>
  </w:body>
</w:document>"#;
    let bytes = build_docx_entries(&[("word/document.xml", document_xml)]);

    let regions = DocxAdapter::extract_regions(&bytes, false).expect("extract regions");

    assert_editable(&regions, "标题");
}

#[test]
fn treats_outline_level_styles_as_headings() {
    let document_xml = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
  <w:body>
    <w:p>
      <w:pPr><w:pStyle w:val="CustomHeading"/></w:pPr>
      <w:r><w:t>结构化标题</w:t></w:r>
    </w:p>
    <w:p><w:r><w:t>正文</w:t></w:r></w:p>
  </w:body>
</w:document>"#;
    let styles_xml = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:styles xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
  <w:style w:type="paragraph" w:styleId="CustomHeading">
    <w:pPr><w:outlineLvl w:val="0"/></w:pPr>
  </w:style>
</w:styles>"#;
    let bytes = build_docx_entries(&[
        ("word/document.xml", document_xml),
        ("word/styles.xml", styles_xml),
    ]);

    let regions = DocxAdapter::extract_regions(&bytes, false).expect("extract regions");

    assert_locked(&regions, "结构化标题");
}

#[test]
fn does_not_lock_cover_like_plain_paragraphs_without_explicit_docx_structure() {
    let document_xml = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
  <w:body>
    <w:p><w:r><w:t>2025年（第18届）</w:t></w:r></w:p>
    <w:p><w:r><w:t>中国高校计算机专业学生</w:t></w:r></w:p>
    <w:p><w:r><w:t>这是一段足够长的正文内容，用于证明后面确实存在文章主体，而且前面的短段落不能仅凭外观被自动锁定。</w:t></w:r></w:p>
  </w:body>
</w:document>"#;
    let bytes = build_docx_entries(&[("word/document.xml", document_xml)]);

    let regions = DocxAdapter::extract_regions(&bytes, false).expect("extract regions");

    assert_editable(&regions, "2025年（第18届）");
    assert_editable(&regions, "中国高校计算机专业学生");
}

#[test]
fn does_not_lock_fill_line_labels_without_explicit_docx_structure() {
    let document_xml = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
  <w:body>
    <w:p>
      <w:r><w:t>作品编号：</w:t></w:r>
      <w:r>
        <w:rPr><w:u w:val="single"/></w:rPr>
        <w:t xml:space="preserve">　　　　</w:t>
      </w:r>
    </w:p>
  </w:body>
</w:document>"#;
    let bytes = build_docx_entries(&[("word/document.xml", document_xml)]);

    let regions = DocxAdapter::extract_regions(&bytes, false).expect("extract regions");

    assert_editable(&regions, "作品编号：");
    assert!(
        !regions.iter().any(|region| {
            region
                .presentation
                .as_ref()
                .and_then(|presentation| presentation.protect_kind.as_deref())
                == Some("fill-line")
        }),
        "expected underlined blank run to remain plain structured text, got:\n{regions:#?}"
    );
}

#[test]
fn does_not_special_case_toc_content_controls_by_gallery_value() {
    let document_xml = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
  <w:body>
    <w:p><w:r><w:t>前</w:t></w:r></w:p>
    <w:sdt>
      <w:sdtPr>
        <w:docPartObj>
          <w:docPartGallery w:val="table of contents"/>
        </w:docPartObj>
      </w:sdtPr>
      <w:sdtContent>
        <w:p><w:r><w:t>目录内容</w:t></w:r></w:p>
      </w:sdtContent>
    </w:sdt>
    <w:p><w:r><w:t>后</w:t></w:r></w:p>
  </w:body>
</w:document>"#;
    let bytes = build_docx_entries(&[("word/document.xml", document_xml)]);

    let regions = DocxAdapter::extract_regions(&bytes, false).expect("extract regions");

    assert!(
        regions.iter().any(|region| {
            region.skip_rewrite
                && region.body.contains("[内容控件]")
                && region
                    .presentation
                    .as_ref()
                    .and_then(|presentation| presentation.protect_kind.as_deref())
                    == Some("content-control")
        }),
        "expected docPartGallery SDT to stay a generic content control, got:\n{regions:#?}"
    );
}

#[test]
fn production_docx_code_does_not_contain_known_semantic_hardcoding_fragments() {
    let mut violations = Vec::new();

    for path in production_docx_sources() {
        let source = fs::read_to_string(&path).expect("read production docx source");
        for fragment in banned_semantic_fragments() {
            if source.contains(fragment) {
                violations.push(format!("{} => {}", path.display(), fragment));
            }
        }
    }

    assert!(
        violations.is_empty(),
        "found semantic hardcoding fragments in production docx code:\n{}",
        violations.join("\n")
    );
}
