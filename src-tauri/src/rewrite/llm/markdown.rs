use crate::adapters::markdown::MarkdownAdapter;
use crate::models::AppSettings;

use super::plans::{
    execute_chunk_plan_serially, has_multiline_skip_region, mask_regions_with_placeholders,
    ChunkRewritePlan,
};

pub(super) async fn rewrite_markdown_chunk_with_client(
    client: &reqwest::Client,
    settings: &AppSettings,
    source_text: &str,
) -> Result<String, String> {
    let plan = plan_markdown_chunk(source_text, settings);
    execute_chunk_plan_serially(client, settings, &plan).await
}

pub(super) fn plan_markdown_chunk(
    source_text: &str,
    settings: &AppSettings,
) -> ChunkRewritePlan {
    let regions = MarkdownAdapter::split_regions(source_text, settings.rewrite_headings);
    if regions.iter().all(|region| !region.skip_rewrite) {
        return ChunkRewritePlan::plain(source_text);
    }
    if has_multiline_skip_region(&regions) {
        return ChunkRewritePlan::from_regions(regions);
    }

    let (masked, placeholders) = mask_regions_with_placeholders(&regions);
    if placeholders.is_empty() {
        return ChunkRewritePlan::plain(source_text);
    }

    ChunkRewritePlan::masked(masked, placeholders)
}
