use crate::models::AppSettings;

use super::plain_support::{build_numbered_multiline_template, finalize_plain_candidate};
use super::prompt::{
    merge_extra_constraints, resolve_system_prompt, EXTRA_CONSTRAINT_NO_MODEL_META,
    EXTRA_CONSTRAINT_NO_MODEL_META_RETRY,
};

const BATCH_MARKER_PREFIX: &str = "<<<LESSAI_ITEM_";

pub(super) async fn rewrite_plain_chunks_with_client(
    client: &reqwest::Client,
    settings: &AppSettings,
    source_texts: &[String],
    extra_constraint: Option<&str>,
) -> Result<Vec<String>, String> {
    super::validate_settings(settings)?;
    if source_texts.is_empty() {
        return Ok(Vec::new());
    }
    if source_texts.len() == 1 {
        return Ok(vec![super::plain::rewrite_plain_chunk_with_client(
            client,
            settings,
            &source_texts[0],
            extra_constraint,
        )
        .await?]);
    }

    let system_prompt = resolve_system_prompt(settings);
    let base_constraint =
        merge_extra_constraints(extra_constraint, &[EXTRA_CONSTRAINT_NO_MODEL_META]);
    let retry_constraint = merge_extra_constraints(
        base_constraint.as_deref(),
        &[EXTRA_CONSTRAINT_NO_MODEL_META_RETRY],
    );
    let mut last_error: Option<String> = None;

    for (attempt, temperature, constraint) in [
        (1usize, settings.temperature, base_constraint.as_deref()),
        (2usize, 0.0, retry_constraint.as_deref()),
    ] {
        let result = rewrite_plain_chunks_with_client_once(
            client,
            settings,
            &system_prompt,
            source_texts,
            constraint,
            temperature,
        )
        .await;

        match result {
            Ok(candidate) => return Ok(candidate),
            Err(error) => {
                last_error = Some(error);
                if attempt >= 2 {
                    break;
                }
            }
        }
    }

    Err(last_error.unwrap_or_else(|| "模型批量改写失败。".to_string()))
}

async fn rewrite_plain_chunks_with_client_once(
    client: &reqwest::Client,
    settings: &AppSettings,
    system_prompt: &str,
    source_texts: &[String],
    extra_constraint: Option<&str>,
    temperature: f32,
) -> Result<Vec<String>, String> {
    let prompt = build_batch_user_prompt(source_texts, extra_constraint);
    let response =
        super::transport::call_chat_model(client, settings, system_prompt, &prompt, temperature)
            .await?;
    let parsed = parse_batch_response(&response, source_texts.len())?;

    source_texts
        .iter()
        .zip(parsed.iter())
        .map(|(source, candidate)| finalize_plain_candidate(source, candidate))
        .collect()
}

fn build_batch_user_prompt(source_texts: &[String], extra_constraint: Option<&str>) -> String {
    let extra_constraint = extra_constraint
        .map(|value| value.trim())
        .filter(|value| !value.is_empty())
        .map(|value| format!("\n- {value}"))
        .unwrap_or_default();
    let blocks = source_texts
        .iter()
        .enumerate()
        .map(|(index, text)| format_batch_block(index + 1, text))
        .collect::<Vec<_>>()
        .join("\n\n");
    let total = source_texts.len();

    format!(
        "请逐项改写下面 {total} 个片段。保留原意与信息密度，尽量减少机械重复感，不要添加解释。\n\n输出要求（必须遵守）：\n- 必须按原顺序输出全部 {total} 项，不得新增、删除、合并、拆分或调换。\n- 每一项必须保留对应的开始/结束标记，且每个标记只能出现一次。\n- 开始/结束标记之间只放该项的改写结果，不要放标题、说明、代码块或额外标注。\n- 如果某项正文中包含形如 @@@1@@@ 的逐行前缀，表示该项必须严格保持行数与行序；这些前缀必须逐字保留并连续输出。\n- 不要输出 Markdown，不要输出这些标记之外的任何内容。{extra_constraint}\n\n原文：\n{blocks}"
    )
}

fn format_batch_block(index: usize, source_text: &str) -> String {
    let begin = begin_marker(index);
    let end = end_marker(index);
    let body = if source_text.contains('\n') || source_text.contains('\r') {
        build_numbered_multiline_template(source_text).0
    } else {
        source_text.to_string()
    };

    format!("{begin}\n{body}\n{end}")
}

fn parse_batch_response(output: &str, expected_items: usize) -> Result<Vec<String>, String> {
    let normalized = output.replace("\r\n", "\n").replace('\r', "\n");
    let lines = normalized.split('\n').collect::<Vec<_>>();
    let mut cursor = 0usize;
    let mut results = Vec::with_capacity(expected_items);

    while cursor < lines.len() && lines[cursor].trim().is_empty() {
        cursor += 1;
    }

    for index in 1..=expected_items {
        let begin = begin_marker(index);
        let end = end_marker(index);

        while cursor < lines.len() && lines[cursor].trim().is_empty() {
            cursor += 1;
        }

        if lines.get(cursor).copied() != Some(begin.as_str()) {
            return Err(format!("模型输出未按要求返回第 {index} 项开始标记。"));
        }
        cursor += 1;

        let mut content = Vec::new();
        while cursor < lines.len() && lines[cursor] != end {
            content.push(lines[cursor].to_string());
            cursor += 1;
        }
        if cursor >= lines.len() {
            return Err(format!("模型输出缺少第 {index} 项结束标记。"));
        }

        results.push(content.join("\n"));
        cursor += 1;
    }

    while cursor < lines.len() {
        if !lines[cursor].trim().is_empty() {
            return Err("模型输出包含多余内容。".to_string());
        }
        cursor += 1;
    }

    Ok(results)
}

fn begin_marker(index: usize) -> String {
    format!("{BATCH_MARKER_PREFIX}{index}_BEGIN>>>")
}

fn end_marker(index: usize) -> String {
    format!("{BATCH_MARKER_PREFIX}{index}_END>>>")
}

#[cfg(test)]
mod tests {
    use super::parse_batch_response;

    #[test]
    fn parse_batch_response_extracts_items_in_order() {
        let output = "\
<<<LESSAI_ITEM_1_BEGIN>>>\n第一项\n<<<LESSAI_ITEM_1_END>>>\n\n<<<LESSAI_ITEM_2_BEGIN>>>\n第二项\n第二行\n<<<LESSAI_ITEM_2_END>>>\n";

        let parsed = parse_batch_response(output, 2).unwrap();

        assert_eq!(parsed, vec!["第一项".to_string(), "第二项\n第二行".to_string()]);
    }
}
