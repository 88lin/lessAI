use std::time::Duration;

use reqwest::header::{AUTHORIZATION, CONTENT_TYPE};
use serde_json::{json, Value};
use std::error::Error as _;

use crate::models::{
    AppSettings, ChunkPreset, DiffSpan, DiffType, PromptPresetId, ProviderCheckResult,
};

const SYSTEM_PROMPT_FALLBACK: &str = "你是一名严谨的中文文本编辑。你的任务是对给定片段进行自然化改写，让表达更像真实人工写作，但必须保持原意、事实、语气和段落层次稳定。不要扩写，不要总结，不要解释，不要输出标题，只输出改写后的正文。";
const SYSTEM_PROMPT_AIGC_V1: &str = include_str!("../../prompt/1.txt");
const SYSTEM_PROMPT_HUMANIZER_ZH: &str = include_str!("../../prompt/2.txt");

fn resolve_system_prompt(settings: &AppSettings) -> String {
    match settings.prompt_preset_id {
        PromptPresetId::AigcV1 => {
            let base = SYSTEM_PROMPT_AIGC_V1.trim();
            let base = if base.is_empty() { SYSTEM_PROMPT_FALLBACK } else { base };
            format!(
                "{}\n\n补充约束：最终输出不要包含“修改后/原文”等标签，只输出改写后的正文。",
                base
            )
        }
        PromptPresetId::HumanizerZh => {
            let base = SYSTEM_PROMPT_HUMANIZER_ZH.trim();
            let base = if base.is_empty() { SYSTEM_PROMPT_FALLBACK } else { base };
            format!(
                "{}\n\n补充约束：最终输出只输出改写后的正文，不要输出标题、列表或解释。",
                base
            )
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SegmentedChunk {
    pub text: String,
    /// 该片段后需要拼回去的分隔符（例如段落间的 "\n\n"）。
    pub separator_after: String,
}

pub fn normalize_text(input: &str) -> String {
    let normalized = input.replace("\r\n", "\n").replace('\r', "\n");
    let mut lines = Vec::new();
    let mut blank_streak = 0usize;

    for raw_line in normalized.lines() {
        let trimmed = raw_line.trim();
        if trimmed.is_empty() {
            blank_streak += 1;
            if blank_streak <= 1 {
                lines.push(String::new());
            }
        } else {
            blank_streak = 0;
            lines.push(trimmed.to_string());
        }
    }

    lines.join("\n").trim().to_string()
}

pub fn segment_text(text: &str, preset: ChunkPreset) -> Vec<SegmentedChunk> {
    // 语义切块的目标是让用户用“边界类型”来选择粒度：
    // - Clause：一小句（逗号/分号等）
    // - Sentence：一整句（句号/问号/感叹号等）
    // - Paragraph：一段话（空行分段）
    //
    // 同时加上硬上限，避免极端长句/长段导致单次调用过重。
    const MAX_CLAUSE_CHARS: usize = 420;
    const MAX_SENTENCE_CHARS: usize = 900;
    const MAX_PARAGRAPH_CHARS: usize = 1_600;

    let paragraphs: Vec<String> = text
        .split("\n\n")
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
        .collect();

    let mut chunks = Vec::new();

    for (paragraph_index, paragraph) in paragraphs.iter().enumerate() {
        let is_last_paragraph = paragraph_index + 1 == paragraphs.len();
        let paragraph_units = segment_paragraph(
            paragraph,
            preset,
            MAX_CLAUSE_CHARS,
            MAX_SENTENCE_CHARS,
            MAX_PARAGRAPH_CHARS,
        );
        let unit_count = paragraph_units.len();

        for (unit_index, unit) in paragraph_units.into_iter().enumerate() {
            let is_last_unit = unit_index + 1 == unit_count;
            let separator_after = if is_last_unit && !is_last_paragraph {
                "\n\n"
            } else {
                ""
            };
            chunks.push(SegmentedChunk {
                text: unit,
                separator_after: separator_after.to_string(),
            });
        }
    }

    if chunks.is_empty() {
        return vec![SegmentedChunk {
            text: text.to_string(),
            separator_after: String::new(),
        }];
    }

    chunks
}

fn segment_paragraph(
    paragraph: &str,
    preset: ChunkPreset,
    max_clause_chars: usize,
    max_sentence_chars: usize,
    max_paragraph_chars: usize,
) -> Vec<String> {
    let trimmed = paragraph.trim();
    if trimmed.is_empty() {
        return Vec::new();
    }

    match preset {
        ChunkPreset::Paragraph => {
            if trimmed.chars().count() <= max_paragraph_chars {
                vec![trimmed.to_string()]
            } else {
                segment_paragraph(
                    trimmed,
                    ChunkPreset::Sentence,
                    max_clause_chars,
                    max_sentence_chars,
                    max_paragraph_chars,
                )
            }
        }
        ChunkPreset::Sentence => split_sentences(trimmed)
            .into_iter()
            .flat_map(|sentence| {
                if sentence.chars().count() <= max_sentence_chars {
                    return vec![sentence];
                }

                // 极端长句：降级到小句边界；再不行就按字符硬切。
                split_clauses(&sentence)
                    .into_iter()
                    .flat_map(|clause| {
                        if clause.chars().count() <= max_sentence_chars {
                            vec![clause]
                        } else {
                            split_by_max_chars(&clause, max_sentence_chars)
                        }
                    })
                    .collect::<Vec<_>>()
            })
            .collect(),
        ChunkPreset::Clause => split_clauses(trimmed)
            .into_iter()
            .flat_map(|clause| {
                if clause.chars().count() <= max_clause_chars {
                    vec![clause]
                } else {
                    split_by_max_chars(&clause, max_clause_chars)
                }
            })
            .collect(),
    }
}

fn split_by_max_chars(text: &str, max_chars: usize) -> Vec<String> {
    if max_chars == 0 {
        return vec![text.trim().to_string()];
    }

    let chars: Vec<char> = text.chars().collect();
    let mut pieces = Vec::new();
    let mut start = 0usize;

    while start < chars.len() {
        let end = (start + max_chars).min(chars.len());
        let slice = chars[start..end].iter().collect::<String>();
        let trimmed = slice.trim();
        if !trimmed.is_empty() {
            pieces.push(trimmed.to_string());
        }
        start = end;
    }

    if pieces.is_empty() {
        vec![text.trim().to_string()]
    } else {
        pieces
    }
}

fn split_sentences(paragraph: &str) -> Vec<String> {
    split_by_boundary(paragraph, BoundaryKind::Sentence)
}

fn split_clauses(paragraph: &str) -> Vec<String> {
    split_by_boundary(paragraph, BoundaryKind::Clause)
}

#[derive(Debug, Clone, Copy)]
enum BoundaryKind {
    Sentence,
    Clause,
}

fn split_by_boundary(text: &str, kind: BoundaryKind) -> Vec<String> {
    let chars: Vec<char> = text.chars().collect();
    let mut units = Vec::new();
    let mut current = String::new();
    let mut index = 0usize;

    while index < chars.len() {
        let ch = chars[index];
        current.push(ch);

        let should_cut = match kind {
            BoundaryKind::Sentence => is_sentence_boundary(&chars, index),
            BoundaryKind::Clause => is_clause_boundary(&chars, index),
        };

        if should_cut {
            while index + 1 < chars.len() && is_closing_punctuation(chars[index + 1]) {
                index += 1;
                current.push(chars[index]);
            }

            let trimmed = current.trim();
            if !trimmed.is_empty() {
                units.push(trimmed.to_string());
            }
            current.clear();
        }

        index += 1;
    }

    if !current.trim().is_empty() {
        units.push(current.trim().to_string());
    }

    if units.is_empty() {
        vec![text.trim().to_string()]
    } else {
        units
    }
}

fn is_sentence_boundary(chars: &[char], index: usize) -> bool {
    let ch = chars[index];
    match ch {
        '。' | '！' | '？' | '!' | '?' | '；' | ';' => true,
        '.' => !is_numeric_punctuation(chars, index),
        _ => false,
    }
}

fn is_clause_boundary(chars: &[char], index: usize) -> bool {
    let ch = chars[index];
    if is_sentence_boundary(chars, index) {
        return true;
    }

    match ch {
        '，' | '、' | '；' | ';' | '：' | ':' => true,
        ',' => !is_numeric_punctuation(chars, index),
        _ => false,
    }
}

fn is_numeric_punctuation(chars: &[char], index: usize) -> bool {
    let ch = chars[index];
    if !matches!(ch, '.' | ',') {
        return false;
    }

    let prev_is_digit = index
        .checked_sub(1)
        .and_then(|prev| chars.get(prev))
        .map(|value| value.is_ascii_digit())
        .unwrap_or(false);
    let next_is_digit = chars
        .get(index + 1)
        .map(|value| value.is_ascii_digit())
        .unwrap_or(false);
    prev_is_digit && next_is_digit
}

fn is_closing_punctuation(ch: char) -> bool {
    matches!(
        ch,
        '"' | '\''
            | '”'
            | '’'
            | '）'
            | ')'
            | '】'
            | ']'
            | '}'
            | '」'
            | '』'
            | '》'
            | '〉'
    )
}

pub fn build_diff(source: &str, candidate: &str) -> Vec<DiffSpan> {
    let source_chars: Vec<char> = source.chars().collect();
    let candidate_chars: Vec<char> = candidate.chars().collect();
    let m = source_chars.len();
    let n = candidate_chars.len();
    let mut dp = vec![vec![0usize; n + 1]; m + 1];

    for i in (0..m).rev() {
        for j in (0..n).rev() {
            if source_chars[i] == candidate_chars[j] {
                dp[i][j] = dp[i + 1][j + 1] + 1;
            } else {
                dp[i][j] = dp[i + 1][j].max(dp[i][j + 1]);
            }
        }
    }

    let mut spans = Vec::new();
    let mut i = 0usize;
    let mut j = 0usize;

    while i < m && j < n {
        if source_chars[i] == candidate_chars[j] {
            push_diff(&mut spans, DiffType::Unchanged, source_chars[i]);
            i += 1;
            j += 1;
        } else if dp[i + 1][j] >= dp[i][j + 1] {
            push_diff(&mut spans, DiffType::Delete, source_chars[i]);
            i += 1;
        } else {
            push_diff(&mut spans, DiffType::Insert, candidate_chars[j]);
            j += 1;
        }
    }

    while i < m {
        push_diff(&mut spans, DiffType::Delete, source_chars[i]);
        i += 1;
    }

    while j < n {
        push_diff(&mut spans, DiffType::Insert, candidate_chars[j]);
        j += 1;
    }

    spans
}

fn push_diff(spans: &mut Vec<DiffSpan>, kind: DiffType, ch: char) {
    if let Some(last) = spans.last_mut() {
        if last.r#type == kind {
            last.text.push(ch);
            return;
        }
    }

    spans.push(DiffSpan {
        r#type: kind,
        text: ch.to_string(),
    });
}

fn format_reqwest_error(error: reqwest::Error) -> String {
    let mut lines = Vec::new();
    lines.push(error.to_string());

    if error.is_timeout() {
        lines.push("提示：请求超时。可以在设置里把“超时（毫秒）”调大（例如 120000）。".to_string());
    }

    if error.is_connect() {
        lines.push("提示：连接失败。常见原因：代理未生效 / DNS 异常 / 证书校验失败 / 网络被拦截。".to_string());
    }

    if error.is_request() {
        lines.push("提示：请求构造失败。请检查 Base URL 格式是否正确（建议只填根地址或 /v1）。".to_string());
    }

    if error.is_body() {
        lines.push("提示：请求体发送失败。可能是网络中断或服务端提前断开连接。".to_string());
    }

    if error.is_decode() {
        lines.push("提示：响应解码失败。可能是接口返回格式不兼容 OpenAI chat/completions。".to_string());
    }

    // 追加底层错误链，帮助定位具体原因（例如证书、DNS、连接拒绝等）
    let mut source = error.source();
    while let Some(cause) = source {
        lines.push(format!("底层错误：{cause}"));
        source = cause.source();
    }

    lines.join("\n")
}

pub async fn test_provider(settings: &AppSettings) -> Result<ProviderCheckResult, String> {
    validate_settings(settings)?;

    let client = reqwest::Client::builder()
        .timeout(Duration::from_millis(settings.timeout_ms))
        .build()
        .map_err(|error| error.to_string())?;

    let response = client
        .get(models_url(&settings.base_url))
        .header(AUTHORIZATION, format!("Bearer {}", settings.api_key))
        .send()
        .await
        .map_err(format_reqwest_error)?;

    if !response.status().is_success() {
        let status = response.status();
        let text = response.text().await.unwrap_or_default();
        return Ok(ProviderCheckResult {
            ok: false,
            message: format!("连接失败：{} {}", status, text),
        });
    }

    Ok(ProviderCheckResult {
        ok: true,
        message: "连接测试通过，模型服务可访问。".to_string(),
    })
}

pub async fn rewrite_chunk(settings: &AppSettings, source_text: &str) -> Result<String, String> {
    validate_settings(settings)?;

    let client = reqwest::Client::builder()
        .timeout(Duration::from_millis(settings.timeout_ms))
        .build()
        .map_err(|error| error.to_string())?;

    let system_prompt = resolve_system_prompt(settings);

    let request_body = json!({
        "model": settings.model,
        "temperature": settings.temperature,
        "messages": [
            {
                "role": "system",
                "content": system_prompt
            },
            {
                "role": "user",
                "content": format!(
                    "请改写下面这段文字。保留原意与信息密度，尽量减少机械重复感。不要添加解释。只输出改写后的正文。\n\n原文：\n{}",
                    source_text
                )
            }
        ]
    });

    let response = client
        .post(chat_url(&settings.base_url))
        .header(AUTHORIZATION, format!("Bearer {}", settings.api_key))
        .header(CONTENT_TYPE, "application/json")
        .json(&request_body)
        .send()
        .await
        .map_err(format_reqwest_error)?;

    if !response.status().is_success() {
        let status = response.status();
        let text = response.text().await.unwrap_or_default();
        return Err(format!("模型调用失败：{} {}", status, text));
    }

    let value: Value = response.json().await.map_err(|error| error.to_string())?;
    let content = extract_content(&value).ok_or_else(|| "模型没有返回有效文本。".to_string())?;
    let sanitized = sanitize_response(&content);

    if sanitized.is_empty() {
        return Err("模型返回内容为空。".to_string());
    }

    Ok(sanitized)
}

fn validate_settings(settings: &AppSettings) -> Result<(), String> {
    if settings.base_url.trim().is_empty() {
        return Err("Base URL 不能为空。".to_string());
    }
    if settings.api_key.trim().is_empty() {
        return Err("API Key 不能为空。".to_string());
    }
    if settings.model.trim().is_empty() {
        return Err("模型名称不能为空。".to_string());
    }

    Ok(())
}

fn normalize_base_url(base_url: &str) -> String {
    base_url.trim().trim_end_matches('/').to_string()
}

fn models_url(base_url: &str) -> String {
    let normalized = normalize_base_url(base_url);
    if normalized.ends_with("/models") {
        normalized
    } else if normalized.ends_with("/v1") {
        format!("{normalized}/models")
    } else {
        format!("{normalized}/v1/models")
    }
}

fn chat_url(base_url: &str) -> String {
    let normalized = normalize_base_url(base_url);
    if normalized.ends_with("/chat/completions") {
        normalized
    } else if normalized.ends_with("/v1") {
        format!("{normalized}/chat/completions")
    } else {
        format!("{normalized}/v1/chat/completions")
    }
}

fn extract_content(value: &Value) -> Option<String> {
    let content = &value["choices"][0]["message"]["content"];

    if let Some(text) = content.as_str() {
        return Some(text.to_string());
    }

    if let Some(items) = content.as_array() {
        let merged = items
            .iter()
            .filter_map(|item| item["text"].as_str())
            .collect::<Vec<_>>()
            .join("");
        if !merged.is_empty() {
            return Some(merged);
        }
    }

    None
}

fn sanitize_response(content: &str) -> String {
    let trimmed = content.trim();
    let without_fences = if trimmed.starts_with("```") {
        trimmed
            .lines()
            .filter(|line| !line.trim_start().starts_with("```"))
            .collect::<Vec<_>>()
            .join("\n")
            .trim()
            .to_string()
    } else {
        trimmed.to_string()
    };

    // 一些提示词会诱导模型输出“修改后：...”或类似标签；这里做一次轻量清理，
    // 避免影响 diff 与导出。
    let mut cleaned = without_fences.trim().to_string();
    for prefix in ["修改后：", "修改后:", "改写后：", "改写后:", "润色后：", "润色后:"] {
        if cleaned.starts_with(prefix) {
            cleaned = cleaned[prefix.len()..].trim_start().to_string();
        }
    }
    if cleaned.starts_with("修改后") {
        let after = cleaned["修改后".len()..].trim_start();
        if let Some(first) = after.chars().next() {
            if matches!(first, ':' | '：' | '-' | '—') {
                cleaned = after[first.len_utf8()..].trim_start().to_string();
            } else if first == '\n' {
                cleaned = after.trim_start().to_string();
            }
        }
    }

    cleaned
}

#[cfg(test)]
mod tests {
    use super::{build_diff, normalize_text, segment_text};
    use crate::models::ChunkPreset;

    #[test]
    fn normalizes_line_endings_and_blank_lines() {
        let input = "第一段\r\n\r\n\r\n 第二段 \r\n";
        assert_eq!(normalize_text(input), "第一段\n\n第二段");
    }

    #[test]
    fn segments_long_paragraphs() {
        let text = "这是第一句。".repeat(80);
        let chunks = segment_text(&text, ChunkPreset::Clause);
        assert!(chunks.len() > 1);
        assert!(chunks.iter().all(|chunk| !chunk.text.is_empty()));
    }

    #[test]
    fn keeps_paragraph_separator_when_splitting_by_sentence() {
        let text = "第一句。第二句。\n\n第三句。";
        let chunks = segment_text(text, ChunkPreset::Sentence);
        assert_eq!(chunks.len(), 3);
        assert_eq!(chunks[0].separator_after, "");
        assert_eq!(chunks[1].separator_after, "\n\n");
        assert_eq!(chunks[2].separator_after, "");
    }

    #[test]
    fn builds_inline_diff() {
        let spans = build_diff("你好", "hollow");
        assert!(spans.iter().any(|span| span.text.contains('你')));
        assert!(spans.iter().any(|span| span.text.contains('h')));
    }
}
