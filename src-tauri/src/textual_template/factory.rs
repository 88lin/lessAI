use crate::{models::DocumentFormat, rewrite_unit::WritebackSlot};

use super::TextTemplate;

pub(crate) fn build_template(
    source_text: &str,
    format: DocumentFormat,
    rewrite_headings: bool,
) -> TextTemplate {
    match format {
        DocumentFormat::PlainText | DocumentFormat::Docx => {
            crate::adapters::plain_text::PlainTextAdapter::build_template(source_text)
        }
        DocumentFormat::Markdown => crate::adapters::markdown::MarkdownAdapter::build_template(
            source_text,
            rewrite_headings,
        ),
        DocumentFormat::Tex => {
            crate::adapters::tex::TexAdapter::build_template(source_text, rewrite_headings)
        }
    }
}

pub(crate) fn build_slots(
    source_text: &str,
    format: DocumentFormat,
    rewrite_headings: bool,
) -> Vec<WritebackSlot> {
    let template = build_template(source_text, format, rewrite_headings);
    super::slots::build_slots(&template).slots
}
