use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppSettings {
    pub base_url: String,
    pub api_key: String,
    pub model: String,
    pub timeout_ms: u64,
    pub temperature: f32,
    pub chunk_preset: ChunkPreset,
    pub rewrite_mode: RewriteMode,
    #[serde(default)]
    pub prompt_preset_id: PromptPresetId,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            base_url: "https://api.openai.com/v1".to_string(),
            api_key: String::new(),
            model: "gpt-4.1-mini".to_string(),
            timeout_ms: 45_000,
            temperature: 0.8,
            chunk_preset: ChunkPreset::Sentence,
            rewrite_mode: RewriteMode::Manual,
            prompt_preset_id: PromptPresetId::default(),
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PromptPresetId {
    AigcV1,
    HumanizerZh,
}

impl Default for PromptPresetId {
    fn default() -> Self {
        Self::HumanizerZh
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ChunkPreset {
    // 兼容历史配置：早期版本可能用 small/medium/large 表达粒度
    #[serde(alias = "small")]
    Clause,
    #[serde(alias = "medium")]
    Sentence,
    #[serde(alias = "large")]
    Paragraph,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum RewriteMode {
    Manual,
    Auto,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ChunkStatus {
    Idle,
    Running,
    Done,
    Failed,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum SuggestionDecision {
    Proposed,
    Applied,
    Dismissed,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum DiffType {
    Unchanged,
    Insert,
    Delete,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum RunningState {
    Idle,
    Running,
    Paused,
    Completed,
    Cancelled,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DiffSpan {
    pub r#type: DiffType,
    pub text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChunkTask {
    pub index: usize,
    pub source_text: String,
    pub separator_after: String,
    pub status: ChunkStatus,
    pub error_message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EditSuggestion {
    pub id: String,
    pub sequence: u64,
    pub chunk_index: usize,
    pub before_text: String,
    pub after_text: String,
    pub diff_spans: Vec<DiffSpan>,
    pub decision: SuggestionDecision,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DocumentSession {
    pub id: String,
    pub title: String,
    pub document_path: String,
    pub source_text: String,
    pub normalized_text: String,
    pub chunks: Vec<ChunkTask>,
    pub suggestions: Vec<EditSuggestion>,
    pub next_suggestion_sequence: u64,
    pub status: RunningState,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderCheckResult {
    pub ok: bool,
    pub message: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RewriteProgress {
    pub session_id: String,
    pub current_chunk: usize,
    pub total_chunks: usize,
    pub mode: RewriteMode,
    pub running_state: RunningState,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ChunkCompletedEvent {
    pub session_id: String,
    pub index: usize,
    pub suggestion_id: String,
    pub suggestion_sequence: u64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RewriteFailedEvent {
    pub session_id: String,
    pub error: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionEvent {
    pub session_id: String,
}
