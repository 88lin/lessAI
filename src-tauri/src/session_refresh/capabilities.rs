use crate::{documents::LoadedDocumentSource, models::DocumentSession};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct SessionCapabilities {
    write_back_supported: bool,
    write_back_block_reason: Option<String>,
    plain_text_editor_safe: bool,
    plain_text_editor_block_reason: Option<String>,
}

impl SessionCapabilities {
    pub(super) fn from_loaded(loaded: &LoadedDocumentSource) -> Self {
        Self {
            write_back_supported: loaded.write_back_supported,
            write_back_block_reason: loaded.write_back_block_reason.clone(),
            plain_text_editor_safe: loaded.plain_text_editor_safe,
            plain_text_editor_block_reason: loaded.plain_text_editor_block_reason.clone(),
        }
    }

    pub(super) fn blocked(reason: &str) -> Self {
        Self {
            write_back_supported: false,
            write_back_block_reason: Some(reason.to_string()),
            plain_text_editor_safe: false,
            plain_text_editor_block_reason: Some(reason.to_string()),
        }
    }
}

pub(super) fn apply_session_capabilities(
    session: &mut DocumentSession,
    capabilities: &SessionCapabilities,
) -> bool {
    let changed = session.write_back_supported != capabilities.write_back_supported
        || session.write_back_block_reason != capabilities.write_back_block_reason
        || session.plain_text_editor_safe != capabilities.plain_text_editor_safe
        || session.plain_text_editor_block_reason != capabilities.plain_text_editor_block_reason;
    if !changed {
        return false;
    }

    session.write_back_supported = capabilities.write_back_supported;
    session.write_back_block_reason = capabilities.write_back_block_reason.clone();
    session.plain_text_editor_safe = capabilities.plain_text_editor_safe;
    session.plain_text_editor_block_reason = capabilities.plain_text_editor_block_reason.clone();
    true
}

#[cfg(test)]
mod tests {
    use super::{apply_session_capabilities, SessionCapabilities};
    use crate::session_refresh::test_support::sample_session;

    #[test]
    fn apply_session_capabilities_updates_all_capability_fields() {
        let mut session = sample_session();

        let changed = apply_session_capabilities(
            &mut session,
            &SessionCapabilities {
                write_back_supported: false,
                write_back_block_reason: Some("write blocked".to_string()),
                plain_text_editor_safe: true,
                plain_text_editor_block_reason: None,
            },
        );

        assert!(changed);
        assert!(!session.write_back_supported);
        assert_eq!(
            session.write_back_block_reason.as_deref(),
            Some("write blocked")
        );
        assert!(session.plain_text_editor_safe);
        assert_eq!(session.plain_text_editor_block_reason, None);
    }

    #[test]
    fn apply_session_capabilities_skips_unchanged_capabilities() {
        let mut session = sample_session();

        let changed = apply_session_capabilities(
            &mut session,
            &SessionCapabilities {
                write_back_supported: true,
                write_back_block_reason: None,
                plain_text_editor_safe: false,
                plain_text_editor_block_reason: Some(
                    "当前文档包含行内锁定内容（如公式、分页符或占位符），暂不支持在纯文本编辑器中直接写回。"
                        .to_string(),
                ),
            },
        );

        assert!(!changed);
    }
}
