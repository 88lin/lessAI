use super::{AppSettings, ChunkPreset};

#[test]
fn rejects_legacy_chunk_preset_aliases() {
    for legacy in ["small", "medium", "large", "question"] {
        let payload = format!("\"{legacy}\"");
        let parsed = serde_json::from_str::<ChunkPreset>(&payload);
        assert!(parsed.is_err(), "legacy preset should be rejected: {legacy}");
    }
}

#[test]
fn accepts_current_chunk_preset_values() {
    assert_eq!(
        serde_json::from_str::<ChunkPreset>("\"clause\"").unwrap(),
        ChunkPreset::Clause
    );
    assert_eq!(
        serde_json::from_str::<ChunkPreset>("\"sentence\"").unwrap(),
        ChunkPreset::Sentence
    );
    assert_eq!(
        serde_json::from_str::<ChunkPreset>("\"paragraph\"").unwrap(),
        ChunkPreset::Paragraph
    );
}

#[test]
fn app_settings_defaults_missing_chunks_per_request_to_one() {
    let parsed = serde_json::from_str::<AppSettings>(
        r#"{
            "baseUrl": "https://api.openai.com/v1",
            "apiKey": "",
            "model": "gpt-4.1-mini",
            "timeoutMs": 45000,
            "temperature": 0.8,
            "chunkPreset": "paragraph",
            "rewriteHeadings": false,
            "rewriteMode": "manual",
            "maxConcurrency": 2,
            "promptPresetId": "humanizer_zh",
            "customPrompts": []
        }"#,
    )
    .unwrap();

    assert_eq!(parsed.chunks_per_request, 1);
}
