#[test]
fn rewrite_session_access_scope_only_blocks_external_entries() {
    assert_eq!(
        super::rewrite_session_active_job_error(super::RewriteSessionAccess::ExternalEntry),
        Some(super::ACTIVE_REWRITE_SESSION_ERROR)
    );
    assert_eq!(
        super::rewrite_session_active_job_error(super::RewriteSessionAccess::ActiveJob),
        None
    );
}
