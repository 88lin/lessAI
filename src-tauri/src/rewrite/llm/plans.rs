use crate::{adapters::TextRegion, models::AppSettings};

pub(super) const PLACEHOLDER_RULE: &str =
    "文本中可能包含形如 ⟦LESSAI_LOCK_1⟧ 的占位符。必须逐字原样保留它们（不得改动/不得删除/不得复制到别处/不得移动顺序）。";

pub(super) struct ChunkRewritePlan {
    segments: Vec<ChunkPlanSegment>,
}

enum ChunkPlanSegment {
    Locked(String),
    Editable(EditableUnit),
}

struct EditableUnit {
    source_text: String,
    restore: RestoreStrategy,
}

enum RestoreStrategy {
    Identity,
    Placeholder(Vec<(String, String)>),
}

impl ChunkRewritePlan {
    pub(super) fn plain(source_text: &str) -> Self {
        if source_text.trim().is_empty() {
            return Self {
                segments: vec![ChunkPlanSegment::Locked(source_text.to_string())],
            };
        }

        Self {
            segments: vec![ChunkPlanSegment::Editable(EditableUnit {
                source_text: source_text.to_string(),
                restore: RestoreStrategy::Identity,
            })],
        }
    }

    pub(super) fn masked(masked_text: String, placeholders: Vec<(String, String)>) -> Self {
        Self {
            segments: vec![ChunkPlanSegment::Editable(EditableUnit {
                source_text: masked_text,
                restore: RestoreStrategy::Placeholder(placeholders),
            })],
        }
    }

    pub(super) fn from_regions(regions: Vec<TextRegion>) -> Self {
        let mut segments = Vec::new();
        for region in regions {
            if region.skip_rewrite || region.body.trim().is_empty() {
                segments.push(ChunkPlanSegment::Locked(region.body));
                continue;
            }

            segments.push(ChunkPlanSegment::Editable(EditableUnit {
                source_text: region.body,
                restore: RestoreStrategy::Identity,
            }));
        }
        Self { segments }
    }

    fn requires_placeholder_rule(&self) -> bool {
        self.segments
            .iter()
            .any(ChunkPlanSegment::requires_placeholder_rule)
    }

    fn collect_editable_sources(&self, out: &mut Vec<String>) {
        for segment in self.segments.iter() {
            if let ChunkPlanSegment::Editable(unit) = segment {
                out.push(unit.source_text.clone());
            }
        }
    }

    fn editable_count(&self) -> usize {
        self.segments
            .iter()
            .filter(|segment| segment.is_editable())
            .count()
    }

    fn rebuild(&self, candidates: &[String]) -> Result<String, String> {
        let mut out = String::new();
        let mut used = 0usize;

        for segment in self.segments.iter() {
            match segment {
                ChunkPlanSegment::Locked(text) => out.push_str(text),
                ChunkPlanSegment::Editable(unit) => {
                    let candidate = candidates
                        .get(used)
                        .ok_or_else(|| "批量改写结果数量不足。".to_string())?;
                    out.push_str(&unit.restore(candidate)?);
                    used += 1;
                }
            }
        }

        if used != candidates.len() {
            return Err("批量改写结果数量异常。".to_string());
        }

        Ok(out)
    }
}

impl ChunkPlanSegment {
    fn is_editable(&self) -> bool {
        matches!(self, Self::Editable(_))
    }

    fn requires_placeholder_rule(&self) -> bool {
        match self {
            Self::Locked(_) => false,
            Self::Editable(unit) => unit.requires_placeholder_rule(),
        }
    }
}

impl EditableUnit {
    fn requires_placeholder_rule(&self) -> bool {
        matches!(self.restore, RestoreStrategy::Placeholder(_))
    }

    fn restore(&self, candidate: &str) -> Result<String, String> {
        match &self.restore {
            RestoreStrategy::Identity => Ok(candidate.to_string()),
            RestoreStrategy::Placeholder(placeholders) => {
                restore_placeholders(candidate, placeholders)
            }
        }
    }
}

pub(super) async fn execute_chunk_plan_serially(
    client: &reqwest::Client,
    settings: &AppSettings,
    plan: &ChunkRewritePlan,
) -> Result<String, String> {
    let mut out = String::new();
    for segment in plan.segments.iter() {
        match segment {
            ChunkPlanSegment::Locked(text) => out.push_str(text),
            ChunkPlanSegment::Editable(unit) => {
                let candidate = super::plain::rewrite_plain_chunk_with_client(
                    client,
                    settings,
                    &unit.source_text,
                    unit.requires_placeholder_rule().then_some(PLACEHOLDER_RULE),
                )
                .await?;
                out.push_str(&unit.restore(&candidate)?);
            }
        }
    }
    Ok(out)
}

pub(super) async fn execute_chunk_plans_batched(
    client: &reqwest::Client,
    settings: &AppSettings,
    plans: &[ChunkRewritePlan],
) -> Result<Vec<String>, String> {
    let mut sources = Vec::new();
    let requires_placeholder_rule = plans
        .iter()
        .any(ChunkRewritePlan::requires_placeholder_rule);
    for plan in plans.iter() {
        plan.collect_editable_sources(&mut sources);
    }
    if sources.is_empty() {
        return Ok(plans
            .iter()
            .map(|plan| plan.rebuild(&[]))
            .collect::<Result<_, _>>()?);
    }

    let rewritten = super::batch::rewrite_plain_chunks_with_client(
        client,
        settings,
        &sources,
        requires_placeholder_rule.then_some(PLACEHOLDER_RULE),
    )
    .await?;

    let mut offset = 0usize;
    let mut rebuilt = Vec::with_capacity(plans.len());
    for plan in plans.iter() {
        let count = plan.editable_count();
        let end = offset.saturating_add(count);
        let candidate_slice = rewritten
            .get(offset..end)
            .ok_or_else(|| "批量改写结果数量与计划不一致。".to_string())?;
        rebuilt.push(plan.rebuild(candidate_slice)?);
        offset = end;
    }

    if offset != rewritten.len() {
        return Err("批量改写结果数量与计划不一致。".to_string());
    }

    Ok(rebuilt)
}

pub(super) fn has_multiline_skip_region(regions: &[TextRegion]) -> bool {
    regions.iter().any(|region| {
        if !region.skip_rewrite {
            return false;
        }
        let trimmed = region.body.trim_end_matches(|ch: char| ch.is_whitespace());
        trimmed.contains('\n') || trimmed.contains('\r')
    })
}

pub(super) fn mask_regions_with_placeholders(
    regions: &[TextRegion],
) -> (String, Vec<(String, String)>) {
    let mut masked = String::new();
    let mut placeholders = Vec::new();
    let mut seq = 1usize;

    for region in regions.iter() {
        if !region.skip_rewrite {
            masked.push_str(&region.body);
            continue;
        }

        let placeholder = format!("⟦LESSAI_LOCK_{seq}⟧");
        seq = seq.saturating_add(1);
        placeholders.push((placeholder.clone(), region.body.clone()));
        masked.push_str(&placeholder);
    }

    (masked, placeholders)
}

fn restore_placeholders(
    candidate: &str,
    placeholders: &[(String, String)],
) -> Result<String, String> {
    let mut search_from = 0usize;
    for (placeholder, _) in placeholders.iter() {
        if candidate.matches(placeholder).count() != 1 {
            return Err("模型输出破坏了锁定占位符边界。".to_string());
        }
        let Some(pos) = candidate[search_from..].find(placeholder) else {
            return Err("模型输出破坏了锁定占位符顺序。".to_string());
        };
        search_from = search_from
            .saturating_add(pos)
            .saturating_add(placeholder.len());
    }

    let mut rebuilt = candidate.to_string();
    for (placeholder, original) in placeholders.iter() {
        rebuilt = rebuilt.replace(placeholder, original);
    }
    Ok(rebuilt)
}

#[cfg(test)]
mod tests {
    use super::{mask_regions_with_placeholders, restore_placeholders};
    use crate::adapters::TextRegion;

    #[test]
    fn restore_placeholders_rejects_missing_placeholder() {
        let error = restore_placeholders(
            "正文",
            &[("⟦LESSAI_LOCK_1⟧".to_string(), "[锁定]".to_string())],
        )
        .unwrap_err();

        assert!(error.contains("占位符"));
    }

    #[test]
    fn mask_regions_with_placeholders_replaces_only_locked_regions() {
        let (masked, placeholders) = mask_regions_with_placeholders(&[
            TextRegion {
                body: "前文".to_string(),
                skip_rewrite: false,
                presentation: None,
            },
            TextRegion {
                body: "[锁定]".to_string(),
                skip_rewrite: true,
                presentation: None,
            },
        ]);

        assert_eq!(masked, "前文⟦LESSAI_LOCK_1⟧");
        assert_eq!(
            placeholders,
            vec![("⟦LESSAI_LOCK_1⟧".to_string(), "[锁定]".to_string())]
        );
    }
}
