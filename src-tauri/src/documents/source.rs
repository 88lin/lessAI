use crate::{adapters, models, rewrite_unit::WritebackSlot};
use std::{fs, path::Path};
use uuid::Uuid;

use super::textual::{load_textual_template_source, path_extension_lower};

pub(super) const PDF_WRITE_BACK_UNSUPPORTED: &str =
    "当前文件为 .pdf：暂不支持写回覆盖（PDF 不是纯文本格式）。请使用“导出”为 .txt 后再进行后续排版。";

pub(crate) fn document_session_id(document_path: &str) -> String {
    let namespace = Uuid::from_bytes([
        0x6c, 0x65, 0x73, 0x73, 0x61, 0x69, 0x2d, 0x64, 0x6f, 0x63, 0x2d, 0x6e, 0x73, 0x2d, 0x30,
        0x31,
    ]);
    Uuid::new_v5(&namespace, document_path.as_bytes()).to_string()
}

pub(crate) fn is_docx_path(path: &Path) -> bool {
    path_extension_lower(path).as_deref() == Some("docx")
}

pub(crate) fn is_pdf_path(path: &Path) -> bool {
    path_extension_lower(path).as_deref() == Some("pdf")
}

pub(crate) struct LoadedDocumentSource {
    pub(crate) source_text: String,
    pub(crate) template_kind: Option<String>,
    pub(crate) template_signature: Option<String>,
    pub(crate) slot_structure_signature: Option<String>,
    pub(crate) template_snapshot: Option<crate::textual_template::TextTemplate>,
    pub(crate) writeback_slots: Vec<WritebackSlot>,
    pub(crate) write_back_supported: bool,
    pub(crate) write_back_block_reason: Option<String>,
    pub(crate) plain_text_editor_safe: bool,
    pub(crate) plain_text_editor_block_reason: Option<String>,
}

pub(crate) fn load_document_source(
    path: &Path,
    rewrite_headings: bool,
) -> Result<LoadedDocumentSource, String> {
    match path_extension_lower(path).as_deref() {
        Some("docx") => load_docx_source(path, rewrite_headings),
        Some("doc") => {
            Err("暂不支持 .doc（老版 Word 二进制格式）。请另存为 .docx 后再导入。".to_string())
        }
        Some("pdf") => load_pdf_source(path),
        _ => load_textual_source(path, rewrite_headings),
    }
}

fn load_docx_source(path: &Path, rewrite_headings: bool) -> Result<LoadedDocumentSource, String> {
    let bytes = fs::read(path).map_err(|error| error.to_string())?;
    let writeback_slots =
        adapters::docx::DocxAdapter::extract_writeback_slots(&bytes, rewrite_headings)?;
    let source_text = writeback_slots
        .iter()
        .map(|slot| format!("{}{}", slot.text, slot.separator_after))
        .collect::<String>();
    if source_text.trim().is_empty() {
        return Err(
            "未从 docx 中抽取到可见文本。该文件可能只有图片/公式/表格，或正文不在 document.xml 中。"
                .to_string(),
        );
    }
    let write_back_block_reason = adapters::docx::DocxAdapter::validate_writeback(&bytes).err();
    let plain_text_editor_block_reason = write_back_block_reason.clone();
    Ok(LoadedDocumentSource {
        source_text,
        template_kind: None,
        template_signature: None,
        slot_structure_signature: None,
        template_snapshot: None,
        writeback_slots,
        write_back_supported: write_back_block_reason.is_none(),
        write_back_block_reason,
        plain_text_editor_safe: plain_text_editor_block_reason.is_none(),
        plain_text_editor_block_reason,
    })
}

fn load_pdf_source(path: &Path) -> Result<LoadedDocumentSource, String> {
    let bytes = fs::read(path).map_err(|error| error.to_string())?;
    let source_text = adapters::pdf::PdfAdapter::extract_text(&bytes)?;
    let writeback_slots = crate::textual_template::factory::build_slots(
        &source_text,
        models::DocumentFormat::PlainText,
        false,
    );
    Ok(LoadedDocumentSource {
        source_text,
        template_kind: None,
        template_signature: None,
        slot_structure_signature: None,
        template_snapshot: None,
        writeback_slots,
        write_back_supported: false,
        write_back_block_reason: Some(PDF_WRITE_BACK_UNSUPPORTED.to_string()),
        plain_text_editor_safe: false,
        plain_text_editor_block_reason: Some(PDF_WRITE_BACK_UNSUPPORTED.to_string()),
    })
}

fn load_textual_source(
    path: &Path,
    rewrite_headings: bool,
) -> Result<LoadedDocumentSource, String> {
    let bytes = fs::read(path).map_err(|error| error.to_string())?;
    let (source_text, template) = load_textual_template_source(path, &bytes, rewrite_headings)?;
    Ok(build_template_loaded_source(source_text, template))
}

fn build_template_loaded_source(
    source_text: String,
    template: crate::textual_template::TextTemplate,
) -> LoadedDocumentSource {
    let template_kind = template.kind.clone();
    let template_signature = template.template_signature.clone();
    let built = crate::textual_template::slots::build_slots(&template);

    LoadedDocumentSource {
        source_text,
        template_kind: Some(template_kind),
        template_signature: Some(template_signature),
        slot_structure_signature: Some(built.slot_structure_signature),
        template_snapshot: Some(template),
        writeback_slots: built.slots,
        write_back_supported: true,
        write_back_block_reason: None,
        plain_text_editor_safe: true,
        plain_text_editor_block_reason: None,
    }
}
