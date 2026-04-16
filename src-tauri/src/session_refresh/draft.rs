use std::path::Path;

use chrono::Utc;

use crate::{
    models::{ChunkPreset, ChunkTask, DocumentSession, RunningState},
    rewrite,
};

use super::{
    capabilities::{apply_session_capabilities, SessionCapabilities},
    RefreshedSession,
};

pub(super) struct SessionRefreshDraft {
    pub(super) session: DocumentSession,
    changed: bool,
}

impl SessionRefreshDraft {
    pub(super) fn new(session: DocumentSession) -> Self {
        Self {
            session,
            changed: false,
        }
    }

    pub(super) fn sync_document_path(&mut self, canonical: &Path) {
        let canonical_path = canonical.to_string_lossy().to_string();
        if self.session.document_path == canonical_path {
            return;
        }
        self.session.document_path = canonical_path;
        self.changed = true;
    }

    pub(super) fn rebuild_chunks(
        &mut self,
        chunks: Vec<ChunkTask>,
        chunk_preset: ChunkPreset,
        rewrite_headings: bool,
    ) {
        self.session.normalized_text = rewrite::normalize_text(&self.session.source_text);
        self.session.chunks = chunks;
        self.session.chunk_preset = Some(chunk_preset);
        self.session.rewrite_headings = Some(rewrite_headings);
        self.session.status = RunningState::Idle;
        self.changed = true;
    }

    pub(super) fn apply_capabilities(&mut self, capabilities: &SessionCapabilities) {
        if apply_session_capabilities(&mut self.session, capabilities) {
            self.changed = true;
        }
    }

    pub(super) fn finish(mut self) -> RefreshedSession {
        if self.changed {
            self.session.updated_at = Utc::now();
        }
        RefreshedSession {
            session: self.session,
            changed: self.changed,
        }
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use chrono::Duration;

    use super::SessionRefreshDraft;
    use crate::{
        models::{ChunkPreset, ChunkStatus, ChunkTask, RunningState},
        session_refresh::test_support::sample_session,
    };

    #[test]
    fn refresh_draft_updates_timestamp_only_after_real_change() {
        let mut session = sample_session();
        session.updated_at -= Duration::seconds(1);
        let original_updated_at = session.updated_at;

        let unchanged = SessionRefreshDraft::new(session.clone()).finish();
        assert!(!unchanged.changed);
        assert_eq!(unchanged.session.updated_at, original_updated_at);

        let mut changed = SessionRefreshDraft::new(session);
        changed.sync_document_path(Path::new("/tmp/canonical/example.docx"));
        let changed = changed.finish();
        assert!(changed.changed);
        assert_eq!(changed.session.document_path, "/tmp/canonical/example.docx");
        assert!(changed.session.updated_at > original_updated_at);
    }

    #[test]
    fn refresh_draft_rebuild_chunks_resets_session_metadata_in_one_place() {
        let mut session = sample_session();
        session.source_text = "第一句。第二句。".to_string();
        session.status = RunningState::Completed;
        session.chunk_preset = Some(ChunkPreset::Paragraph);
        session.rewrite_headings = Some(false);

        let mut draft = SessionRefreshDraft::new(session);
        draft.rebuild_chunks(
            vec![ChunkTask {
                index: 0,
                source_text: "第一句。第二句。".to_string(),
                separator_after: String::new(),
                skip_rewrite: false,
                presentation: None,
                status: ChunkStatus::Idle,
                error_message: None,
            }],
            ChunkPreset::Sentence,
            true,
        );
        let refreshed = draft.finish();

        assert!(refreshed.changed);
        assert_eq!(refreshed.session.normalized_text, "第一句。第二句。");
        assert_eq!(refreshed.session.chunks.len(), 1);
        assert_eq!(refreshed.session.chunk_preset, Some(ChunkPreset::Sentence));
        assert_eq!(refreshed.session.rewrite_headings, Some(true));
        assert_eq!(refreshed.session.status, RunningState::Idle);
    }
}
