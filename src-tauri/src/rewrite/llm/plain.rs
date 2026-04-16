use crate::models::AppSettings;

use super::super::text::split_line_skeleton;
use super::plain_support::{build_numbered_multiline_template, finalize_plain_candidate};
use super::prompt::{
    merge_extra_constraints, resolve_system_prompt, EXTRA_CONSTRAINT_NO_MODEL_META,
};

async fn call_rewrite_model(
    client: &reqwest::Client,
    settings: &AppSettings,
    system_prompt: &str,
    user_prompt: &str,
    temperature: f32,
) -> Result<String, String> {
    super::transport::call_chat_model(client, settings, system_prompt, user_prompt, temperature)
        .await
}

async fn rewrite_plain_chunk_with_client_once(
    client: &reqwest::Client,
    settings: &AppSettings,
    system_prompt: &str,
    source_text: &str,
    extra_constraint: Option<&str>,
    temperature: f32,
) -> Result<String, String> {
    if source_text.trim().is_empty() {
        return Ok(source_text.to_string());
    }

    let user_prompt = if source_text.contains('\n') || source_text.contains('\r') {
        build_multiline_rewrite_prompt(source_text, extra_constraint).0
    } else {
        let (_, core, _) = split_line_skeleton(source_text);
        if core.trim().is_empty() {
            return Ok(source_text.to_string());
        }
        build_singleline_rewrite_prompt(&core, extra_constraint)
    };
    let candidate =
        call_rewrite_model(client, settings, system_prompt, &user_prompt, temperature).await?;

    finalize_plain_candidate(source_text, &candidate)
}

pub(super) async fn rewrite_plain_chunk_with_client(
    client: &reqwest::Client,
    settings: &AppSettings,
    source_text: &str,
    extra_constraint: Option<&str>,
) -> Result<String, String> {
    super::validate_settings(settings)?;

    let system_prompt = resolve_system_prompt(settings);
    let constraint = merge_extra_constraints(extra_constraint, &[EXTRA_CONSTRAINT_NO_MODEL_META]);

    rewrite_plain_chunk_with_client_once(
        client,
        settings,
        &system_prompt,
        source_text,
        constraint.as_deref(),
        settings.temperature,
    )
    .await
}

fn build_singleline_rewrite_prompt(source_body: &str, extra_constraint: Option<&str>) -> String {
    let extra_constraint = extra_constraint
        .map(|value| value.trim())
        .filter(|value| !value.is_empty())
        .map(|value| format!("\n\n额外约束（必须遵守）：\n- {value}"))
        .unwrap_or_default();
    format!(
        "请改写下面这段文字。保留原意与信息密度，尽量减少机械重复感。不要添加解释。\n\n格式要求（必须遵守）：\n- 严格保持原文的换行/空行/缩进/列表符号与标点风格，不要新增或删除换行。\n- 不要输出 Markdown（尤其不要使用行尾两个空格来制造换行）。\n- 只输出改写后的正文，不要输出任何标签或解释。{extra_constraint}\n\n原文：\n{source_body}"
    )
}

fn build_multiline_rewrite_prompt(
    source_body: &str,
    extra_constraint: Option<&str>,
) -> (String, usize) {
    let (template, expected) = build_numbered_multiline_template(source_body);

    let extra_constraint = extra_constraint
        .map(|value| value.trim())
        .filter(|value| !value.is_empty())
        .map(|value| format!("\n- {value}"))
        .unwrap_or_default();

    let prompt = format!(
        "请对下面的文本进行改写，让表达更自然，但必须【严格保持行结构不变】。\n\n输入格式：每行以 @@@序号@@@ 开头（序号从 1 开始）。\n输出要求（必须遵守）：\n- 必须输出【相同数量】的行，行序号必须从 1 到 {expected} 连续且不重复。\n- 每行必须保留对应的 @@@序号@@@ 前缀，且不得新增、删除、合并或拆分任何一行。\n- 每行改写后的内容必须在同一行内，不得包含换行符。\n- 空行（只有前缀没有内容）必须原样输出为空行（仍保留前缀）。\n- 不要输出 Markdown/代码块/解释/标题，只输出这些行。{extra_constraint}\n\n原文：\n{template}"
    );

    (prompt, expected)
}
