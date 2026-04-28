pub mod docx;
pub mod markdown;
pub mod pdf;
pub mod plain_text;
pub mod tex;

use crate::models::TextPresentation;
use crate::rewrite_unit::WritebackSlotRole;
use crate::text_boundaries::split_text_and_trailing_separator;
use crate::textual_template::models::{TextRegionSplitMode, TextTemplateRegion};

#[derive(Debug, Clone)]
pub struct TextRegion {
    pub body: String,
    pub skip_rewrite: bool,
    pub role: WritebackSlotRole,
    pub split_mode: TextRegionSplitMode,
    pub presentation: Option<TextPresentation>,
}

impl TextRegion {
    pub fn editable(body: impl Into<String>) -> Self {
        Self {
            body: body.into(),
            skip_rewrite: false,
            role: WritebackSlotRole::EditableText,
            split_mode: TextRegionSplitMode::BoundaryAware,
            presentation: None,
        }
    }

    pub fn locked_text(body: impl Into<String>) -> Self {
        Self {
            body: body.into(),
            skip_rewrite: true,
            role: WritebackSlotRole::LockedText,
            split_mode: TextRegionSplitMode::Atomic,
            presentation: None,
        }
    }

    pub fn syntax_token(body: impl Into<String>) -> Self {
        Self {
            role: WritebackSlotRole::SyntaxToken,
            ..Self::locked_text(body)
        }
    }

    pub fn inline_object(body: impl Into<String>) -> Self {
        Self {
            role: WritebackSlotRole::InlineObject,
            ..Self::locked_text(body)
        }
    }

    pub fn with_presentation(mut self, presentation: Option<TextPresentation>) -> Self {
        self.presentation = presentation;
        self
    }

    pub fn with_split_mode(mut self, split_mode: TextRegionSplitMode) -> Self {
        self.split_mode = split_mode;
        self
    }

    pub(crate) fn into_template_region(
        self,
        block_anchor: &str,
        region_index: usize,
    ) -> TextTemplateRegion {
        let (text, separator_after) = split_text_and_trailing_separator(&self.body);

        TextTemplateRegion {
            anchor: format!("{block_anchor}:r{region_index}"),
            text,
            editable: !self.skip_rewrite,
            role: self.role,
            presentation: self.presentation,
            split_mode: self.split_mode,
            separator_after,
        }
    }
}
