use std::{
    collections::HashMap,
    io::{Cursor, Read},
};

use quick_xml::{events::Event, Reader};
use zip::ZipArchive;

use super::{
    numbering::{parse_numbering_xml, NumberingDefinitions},
    styles::{parse_styles_xml, ParagraphStyles},
    xml::{attr_value, local_name},
};

#[derive(Debug, Default)]
pub(super) struct DocxSupportData {
    pub(super) hyperlink_targets: HashMap<String, String>,
    pub(super) numbering: NumberingDefinitions,
    pub(super) styles: ParagraphStyles,
}

#[derive(Debug)]
pub(super) struct DocxParts {
    pub(super) document_xml: String,
    pub(super) relationships_xml: Option<String>,
    pub(super) numbering_xml: Option<String>,
    pub(super) styles_xml: Option<String>,
}

pub(super) struct LoadedDocx {
    pub(super) document_xml: String,
    pub(super) support: DocxSupportData,
}

pub(super) fn load_docx_document(docx_bytes: &[u8]) -> Result<LoadedDocx, String> {
    let parts = load_docx_parts(docx_bytes)?;
    let support = build_docx_support_data(&parts)?;
    Ok(LoadedDocx {
        document_xml: parts.document_xml,
        support,
    })
}

pub(super) fn load_docx_parts(docx_bytes: &[u8]) -> Result<DocxParts, String> {
    if docx_bytes.is_empty() {
        return Err("docx 文件为空。".to_string());
    }

    let cursor = Cursor::new(docx_bytes);
    let mut archive = ZipArchive::new(cursor).map_err(format_docx_zip_error)?;

    Ok(DocxParts {
        document_xml: read_required_xml_entry(
            &mut archive,
            "word/document.xml",
            "docx 缺少 word/document.xml，无法读取正文。",
        )?,
        relationships_xml: read_optional_xml_entry(&mut archive, "word/_rels/document.xml.rels")?,
        numbering_xml: read_optional_xml_entry(&mut archive, "word/numbering.xml")?,
        styles_xml: read_optional_xml_entry(&mut archive, "word/styles.xml")?,
    })
}

pub(super) fn format_docx_zip_error(error: zip::result::ZipError) -> String {
    let detail = error.to_string();
    let lower = detail.to_ascii_lowercase();
    if lower.contains("eocd") {
        return format!(
            "无法解析 docx：压缩包尾部目录（EOCD）缺失或损坏。文件可能未下载完整、被截断，或并非真实 .docx。底层错误：{detail}"
        );
    }
    if lower.contains("invalid zip archive") {
        return format!(
            "无法解析 docx：文件不是有效的 zip/docx 结构，可能是扩展名被改成 .docx。底层错误：{detail}"
        );
    }
    format!("无法解析 docx（zip 结构错误）：{detail}")
}

fn build_docx_support_data(parts: &DocxParts) -> Result<DocxSupportData, String> {
    Ok(DocxSupportData {
        hyperlink_targets: match parts.relationships_xml.as_deref() {
            Some(xml) => parse_relationship_targets(xml)?,
            None => HashMap::new(),
        },
        numbering: match parts.numbering_xml.as_deref() {
            Some(xml) => parse_numbering_xml(xml)?,
            None => NumberingDefinitions::default(),
        },
        styles: match parts.styles_xml.as_deref() {
            Some(xml) => parse_styles_xml(xml)?,
            None => ParagraphStyles::default(),
        },
    })
}

fn read_required_xml_entry(
    archive: &mut ZipArchive<Cursor<&[u8]>>,
    name: &str,
    missing_error: &str,
) -> Result<String, String> {
    let mut file = archive
        .by_name(name)
        .map_err(|_| missing_error.to_string())?;
    let mut xml = String::new();
    file.read_to_string(&mut xml)
        .map_err(|error| format!("读取 {name} 失败：{error}"))?;
    Ok(xml)
}

fn read_optional_xml_entry(
    archive: &mut ZipArchive<Cursor<&[u8]>>,
    name: &str,
) -> Result<Option<String>, String> {
    let mut file = match archive.by_name(name) {
        Ok(file) => file,
        Err(_) => return Ok(None),
    };
    let mut xml = String::new();
    file.read_to_string(&mut xml)
        .map_err(|error| format!("读取 {name} 失败：{error}"))?;
    Ok(Some(xml))
}

fn parse_relationship_targets(xml: &str) -> Result<HashMap<String, String>, String> {
    let mut reader = Reader::from_str(xml);
    reader.config_mut().trim_text(true);

    let mut buf = Vec::new();
    let mut targets = HashMap::new();

    loop {
        let event = match reader.read_event_into(&mut buf) {
            Ok(event) => event.into_owned(),
            Err(error) => return Err(format!("解析 document.xml.rels 失败：{error}")),
        };

        match event {
            Event::Start(e) | Event::Empty(e) => {
                if local_name(e.name().as_ref()) != b"Relationship" {
                    buf.clear();
                    continue;
                }
                let relationship_type = attr_value(&e, b"Type");
                if !relationship_type
                    .as_deref()
                    .is_some_and(|value| value.ends_with("/hyperlink"))
                {
                    buf.clear();
                    continue;
                }
                let id = attr_value(&e, b"Id")
                    .ok_or_else(|| "document.xml.rels 中的超链接关系缺少 Id。".to_string())?;
                let target = attr_value(&e, b"Target").ok_or_else(|| {
                    format!("document.xml.rels 中的超链接关系 {id} 缺少 Target。")
                })?;
                targets.insert(id, target);
            }
            Event::Eof => break,
            _ => {}
        }

        buf.clear();
    }

    Ok(targets)
}
