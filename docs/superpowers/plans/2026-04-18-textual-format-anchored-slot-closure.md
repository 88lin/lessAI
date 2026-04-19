# Textual Format Anchored Slot Closure Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 将 `txt / md / tex` 提升到与 `docx` 同级的 anchored-slot 闭环，确保 `1 个用户块 = 1 个 rewrite unit`、`1 个 batch = 1 次模型调用`、AI 改写只通过 `SlotUpdate[]` 落地，并且所有可改写内容都能被同一条安全写回链严格验证。

**Architecture:** 新增共享 `textual_template` 基础设施，把文本格式统一拆成 `FormatTemplate -> AnchoredSlot -> RewriteUnit -> SlotWriteback` 四段主链。按纵向切片实施：先搭共享模板与签名基础设施，再先打通 `txt` 全闭环，再迁移 `markdown`、`tex`，最后把 `writeback / refresh / selection` 全部收口到模板+slot 真相并删除旧的 `TextRegion -> 整篇文本覆盖` 生产链。

**Tech Stack:** Rust, Tauri, serde, 现有 `rewrite_unit` 管线, Windows `cargo.exe`, pnpm typecheck

---

## Planned File Map

- Create: `src-tauri/src/textual_template/mod.rs`
  统一导出文本模板模型、签名、slot 构造、重建、写回校验。
- Create: `src-tauri/src/textual_template/models.rs`
  定义 `TextTemplate`, `TextTemplateBlock`, `TextTemplateRegion`, `TextTemplateKind` 等共享模型。
- Create: `src-tauri/src/textual_template/signature.rs`
  负责 `template_signature` 与 `slot_structure_signature` 计算。
- Create: `src-tauri/src/textual_template/slots.rs`
  将模板转成 `WritebackSlot[]`，并以稳定 anchor 计算 slot 结构签名。
- Create: `src-tauri/src/textual_template/rebuild.rs`
  从模板快照和更新后的 slots 重建最终文本。
- Create: `src-tauri/src/textual_template/validate.rs`
  校验模板签名、slot 结构签名、slot 更新范围与 editable 边界。
- Create: `src-tauri/src/adapters/plain_text.rs`
  为纯文本提供 `build_template(...)` 主入口。
- Create: `src-tauri/src/adapters/markdown/mod.rs`
  Markdown 新入口，避免继续把新增逻辑塞进超长单文件。
- Create: `src-tauri/src/adapters/markdown/blocks.rs`
  Markdown 块级扫描：heading、paragraph、quote、list item、locked block。
- Create: `src-tauri/src/adapters/markdown/inline.rs`
  Markdown 行内锁定片段扫描：inline code、link shell、URL、math、html。
- Create: `src-tauri/src/adapters/markdown/template.rs`
  块/行内扫描结果转 `TextTemplate`。
- Create: `src-tauri/src/adapters/markdown/tests.rs`
  Markdown 模板与锚点回归测试。
- Create: `src-tauri/src/adapters/tex/mod.rs`
  TeX 新入口，避免继续在现有大文件上堆叠逻辑。
- Create: `src-tauri/src/adapters/tex/blocks.rs`
  TeX 块级扫描：paragraph、command block、environment block、math block、locked block。
- Create: `src-tauri/src/adapters/tex/commands.rs`
  命令壳、环境壳、数学、注释、raw 内容扫描。
- Create: `src-tauri/src/adapters/tex/template.rs`
  TeX 扫描结果转 `TextTemplate`。
- Create: `src-tauri/src/adapters/tex/tests.rs`
  TeX 模板与锚点回归测试。
- Modify: `src-tauri/src/adapters/mod.rs`
  导出 `plain_text`、新的 `markdown`/`tex` 模块，以及共享的模板输入/输出类型。
- Modify: `src-tauri/src/documents/source.rs`
  所有文本格式导入都改走 `build_template -> build_slots -> build_rewrite_units`。
- Modify: `src-tauri/src/documents/writeback.rs`
  文本格式写回改为 `Slots` 路径，写回前必须重建模板并校验签名。
- Modify: `src-tauri/src/documents.rs`
  删除旧的文本格式生产入口导出，只保留新主链。
- Modify: `src-tauri/src/models.rs`
  `DocumentSession` 增加模板签名和模板快照字段，全部带 `#[serde(default)]`。
- Modify: `src-tauri/src/session_builder.rs`
  clean session 构建时持久化模板签名、slot 结构签名与模板快照。
- Modify: `src-tauri/src/session_refresh.rs`
  刷新逻辑改为看模板签名和 slot 结构签名，不再只比对拼接文本。
- Modify: `src-tauri/src/session_refresh/rules.rs`
  将 `Keep / Rebuild / Block` 判定切换到模板闭环。
- Modify: `src-tauri/src/session_refresh/draft.rs`
  重建会话结构时同步模板相关字段。
- Modify: `src-tauri/src/rewrite_writeback.rs`
  文本格式最终写回统一走 `Slots` 投影，不再回落为整篇字符串覆盖。
- Modify: `src-tauri/src/rewrite/llm/selection.rs`
  选区改写也改走格式模板和 slot 流，不再临时 `TextRegion` 切块。
- Modify: `src-tauri/src/documents_source_tests.rs`
  替换旧 `writeback_slots_from_regions` 生产性断言，改测模板锚点和 slot 结构。
- Modify: `src-tauri/src/documents_writeback_tests.rs`
  增加文本格式 slot 级写回校验与重建回归。
- Modify: `src-tauri/src/session_refresh/refresh_structure_tests.rs`
  增加模板签名变化、slot 结构变化的 `Rebuild / Block` 测试。
- Modify: `src-tauri/src/rewrite/llm_regression_tests.rs`
  增加 selection rewrite 使用模板/slot 的回归。
- Modify: `src-tauri/src/rewrite_writeback_tests.rs`
  增加文本格式 applied slot projection 写回验证。

前端本轮不改交互模型，继续只消费 `writebackSlots` 和 `rewriteUnits`；模板快照与签名仅作为后端闭环真相持久化。

### Task 1: 建立共享 `textual_template` 主干并把模板元数据写入 session

**Files:**
- Create: `src-tauri/src/textual_template/mod.rs`
- Create: `src-tauri/src/textual_template/models.rs`
- Create: `src-tauri/src/textual_template/signature.rs`
- Create: `src-tauri/src/textual_template/slots.rs`
- Create: `src-tauri/src/textual_template/rebuild.rs`
- Create: `src-tauri/src/textual_template/validate.rs`
- Modify: `src-tauri/src/models.rs`
- Modify: `src-tauri/src/documents/source.rs`
- Modify: `src-tauri/src/session_builder.rs`
- Modify: `src-tauri/src/documents_source_tests.rs`

- [ ] **Step 1: 先写失败测试，钉死模板元数据必须进入 `LoadedDocumentSource` 和 `DocumentSession`**

```rust
#[test]
fn build_clean_session_persists_textual_template_metadata() {
    let template = crate::textual_template::models::TextTemplate::single_paragraph(
        "plain_text",
        "txt:p0",
        "第一段\n\n",
    );
    let built = crate::textual_template::slots::build_slots(&template);
    let loaded = LoadedDocumentSource {
        source_text: "第一段\n\n".to_string(),
        template_kind: Some("plain_text".to_string()),
        template_signature: Some(template.template_signature.clone()),
        slot_structure_signature: Some(built.slot_structure_signature.clone()),
        template_snapshot: Some(template.clone()),
        writeback_slots: built.slots,
        write_back_supported: true,
        write_back_block_reason: None,
        plain_text_editor_safe: true,
        plain_text_editor_block_reason: None,
    };

    let session = build_clean_session(CleanSessionBuildInput {
        session_id: "session-1".to_string(),
        canonical_path: std::path::Path::new("/tmp/example.txt"),
        document_path: "/tmp/example.txt".to_string(),
        loaded,
        source_snapshot: None,
        segmentation_preset: SegmentationPreset::Paragraph,
        rewrite_headings: false,
        created_at: Utc::now(),
    });

    assert_eq!(session.template_kind.as_deref(), Some("plain_text"));
    assert!(session.template_snapshot.is_some());
    assert!(session.template_signature.is_some());
    assert!(session.slot_structure_signature.is_some());
}
```

- [ ] **Step 2: 运行定向测试，确认当前因为缺少模板字段和模块而失败**

Run:

```bash
/mnt/c/Windows/System32/cmd.exe /C "cd /d E:\Code\LessAI\src-tauri && cargo test build_clean_session_persists_textual_template_metadata -- --exact"
```

Expected: FAIL，报 `template_kind` / `template_signature` / `template_snapshot` 字段不存在，或 `textual_template` 模块不存在。

- [ ] **Step 3: 实现共享模板模型与 slot 构造基础设施**

```rust
// src-tauri/src/textual_template/models.rs
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TextTemplate {
    pub kind: String,
    pub blocks: Vec<TextTemplateBlock>,
    pub template_signature: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TextTemplateBlock {
    pub anchor: String,
    pub kind: String,
    pub regions: Vec<TextTemplateRegion>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TextTemplateRegion {
    pub anchor: String,
    pub text: String,
    pub editable: bool,
    pub role: WritebackSlotRole,
    pub presentation: Option<TextPresentation>,
    pub separator_after: String,
}
```

- [ ] **Step 4: 把模板元数据线程化到 `LoadedDocumentSource` 和 `DocumentSession`，并全部加默认值**

```rust
pub(crate) struct LoadedDocumentSource {
    pub(crate) source_text: String,
    pub(crate) template_kind: Option<String>,
    pub(crate) template_signature: Option<String>,
    pub(crate) slot_structure_signature: Option<String>,
    pub(crate) template_snapshot: Option<TextTemplate>,
    pub(crate) writeback_slots: Vec<WritebackSlot>,
    // 现有 capability 字段保持不变
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DocumentSession {
    // 现有字段保持不变
    #[serde(default)]
    pub template_kind: Option<String>,
    #[serde(default)]
    pub template_signature: Option<String>,
    #[serde(default)]
    pub slot_structure_signature: Option<String>,
    #[serde(default)]
    pub template_snapshot: Option<TextTemplate>,
}
```

- [ ] **Step 5: 重新运行 Task 1 的定向测试**

Run:

```bash
/mnt/c/Windows/System32/cmd.exe /C "cd /d E:\Code\LessAI\src-tauri && cargo test build_clean_session_persists_textual_template_metadata -- --exact"
```

Expected: PASS，且现有 `session_builder` 测试不应新增反序列化错误。

- [ ] **Step 6: 提交共享基础设施骨架**

```bash
git add src-tauri/src/textual_template src-tauri/src/models.rs src-tauri/src/documents/source.rs src-tauri/src/session_builder.rs src-tauri/src/documents_source_tests.rs
git commit -m "搭建文本模板闭环基础设施"
```

### Task 2: 打通 `txt` 的完整 anchored-slot 闭环

**Files:**
- Create: `src-tauri/src/adapters/plain_text.rs`
- Modify: `src-tauri/src/adapters/mod.rs`
- Modify: `src-tauri/src/documents/source.rs`
- Modify: `src-tauri/src/documents/writeback.rs`
- Modify: `src-tauri/src/session_refresh.rs`
- Modify: `src-tauri/src/session_refresh/rules.rs`
- Modify: `src-tauri/src/session_refresh/refresh_structure_tests.rs`
- Modify: `src-tauri/src/documents_writeback_tests.rs`
- Modify: `src-tauri/src/documents_source_tests.rs`

- [ ] **Step 1: 写失败测试，钉死 `txt` 必须使用段落 anchor、slot 结构签名与 slot 级写回**

```rust
#[test]
fn load_plain_text_source_builds_stable_paragraph_anchors() {
    let (root, path) = write_temp_file("plain-template", "txt", "第一段\n\n第二段".as_bytes());
    let loaded = load_document_source(&path, false).expect("load txt");

    let anchors = loaded
        .writeback_slots
        .iter()
        .map(|slot| slot.anchor.clone().unwrap_or_default())
        .collect::<Vec<_>>();

    assert_eq!(anchors, vec!["txt:p0:r0:s0", "txt:p1:r0:s0"]);
    assert!(loaded.template_signature.is_some());
    assert!(loaded.slot_structure_signature.is_some());
    cleanup_dir(&root);
}

#[test]
fn validate_plain_text_slot_writeback_rejects_changed_anchor_order() {
    let (root, path) = write_temp_file("plain-writeback", "txt", "第一段\n\n第二段".as_bytes());
    let loaded = load_document_source(&path, false).expect("load txt");
    let snapshot = capture_document_snapshot(&path).expect("snapshot");
    let mut updated = loaded.writeback_slots.clone();
    updated.swap(0, 1);

    let error = execute_document_writeback(
        &path,
        &loaded.source_text,
        Some(&snapshot),
        DocumentWriteback::Slots(&updated),
        WritebackMode::Validate,
    )
    .expect_err("anchor reorder must fail");

    assert!(error.contains("结构") || error.contains("anchor"));
    cleanup_dir(&root);
}
```

- [ ] **Step 2: 运行定向测试，确认当前 `txt` 仍走旧链而失败**

Run:

```bash
/mnt/c/Windows/System32/cmd.exe /C "cd /d E:\Code\LessAI\src-tauri && cargo test load_plain_text_source_builds_stable_paragraph_anchors -- --exact"
/mnt/c/Windows/System32/cmd.exe /C "cd /d E:\Code\LessAI\src-tauri && cargo test validate_plain_text_slot_writeback_rejects_changed_anchor_order -- --exact"
```

Expected: FAIL，前者拿不到 anchor 或签名，后者会因为文本格式不支持 `Slots` 写回而失败。

- [ ] **Step 3: 实现 `PlainTextAdapter::build_template(...)`，按段落生成稳定模板与 slots**

```rust
impl PlainTextAdapter {
    pub fn build_template(text: &str) -> TextTemplate {
        let blocks = split_text_chunks_by_paragraph_separator(text)
            .into_iter()
            .enumerate()
            .map(|(paragraph_index, chunk)| {
                let (body, separator_after) = split_region_body_and_separator(chunk);
                TextTemplateBlock {
                    anchor: format!("txt:p{paragraph_index}"),
                    kind: "paragraph".to_string(),
                    regions: vec![TextTemplateRegion {
                        anchor: format!("txt:p{paragraph_index}:r0"),
                        text: body,
                        editable: true,
                        role: WritebackSlotRole::EditableText,
                        presentation: None,
                        separator_after,
                    }],
                }
            })
            .collect::<Vec<_>>();
        finalize_template("plain_text", blocks)
    }
}
```

- [ ] **Step 4: 让 `txt` 导入、写回、refresh 全部使用模板闭环**

```rust
let template = PlainTextAdapter::build_template(&source_text);
let built = textual_template::slots::build_slots(&template);
Ok(LoadedDocumentSource {
    source_text,
    template_kind: Some(template.kind.clone()),
    template_signature: Some(template.template_signature.clone()),
    slot_structure_signature: Some(built.slot_structure_signature.clone()),
    template_snapshot: Some(template),
    writeback_slots: built.slots,
    write_back_supported: true,
    write_back_block_reason: None,
    plain_text_editor_safe: true,
    plain_text_editor_block_reason: None,
})
```

- [ ] **Step 5: 重跑 `txt` 相关测试，确认导入、写回、refresh 三条链闭环**

Run:

```bash
/mnt/c/Windows/System32/cmd.exe /C "cd /d E:\Code\LessAI\src-tauri && cargo test plain_text -- --nocapture"
```

Expected: PASS，包含 `documents_source_tests`、`documents_writeback_tests`、`refresh_structure_tests` 中新增的 `plain_text` 用例。

- [ ] **Step 6: 提交 `txt` 纵向切片**

```bash
git add src-tauri/src/adapters/plain_text.rs src-tauri/src/adapters/mod.rs src-tauri/src/documents/source.rs src-tauri/src/documents/writeback.rs src-tauri/src/session_refresh.rs src-tauri/src/session_refresh/rules.rs src-tauri/src/session_refresh/refresh_structure_tests.rs src-tauri/src/documents_writeback_tests.rs src-tauri/src/documents_source_tests.rs
git commit -m "打通纯文本模板化写回闭环"
```

### Task 3: 将 Markdown 迁移到模板主链，并拆分超长适配器文件

**Files:**
- Create: `src-tauri/src/adapters/markdown/mod.rs`
- Create: `src-tauri/src/adapters/markdown/blocks.rs`
- Create: `src-tauri/src/adapters/markdown/inline.rs`
- Create: `src-tauri/src/adapters/markdown/template.rs`
- Create: `src-tauri/src/adapters/markdown/tests.rs`
- Delete: `src-tauri/src/adapters/markdown.rs`
- Modify: `src-tauri/src/adapters/mod.rs`
- Modify: `src-tauri/src/documents/source.rs`
- Modify: `src-tauri/src/rewrite/llm/selection.rs`

- [ ] **Step 1: 写失败测试，钉死 Markdown 的块级/行内锁定边界和稳定 anchor**

```rust
#[test]
fn markdown_template_locks_url_inline_code_and_front_matter() {
    let text = "---\ntitle: Demo\n---\n\n## 标题\n访问 https://example.com ，并执行 `cargo test`。";
    let template = MarkdownAdapter::build_template(text, false);
    let built = crate::textual_template::slots::build_slots(&template);

    assert!(built.slots.iter().any(|slot| slot.anchor.as_deref() == Some("md:b0:r0:s0") && !slot.editable));
    assert!(built.slots.iter().any(|slot| slot.text == "https://example.com" && !slot.editable));
    assert!(built.slots.iter().any(|slot| slot.text == "`cargo test`" && !slot.editable));
}

#[test]
fn markdown_template_rebuild_round_trips_original_text() {
    let text = "1. 第一项\n2. 第二项含 [链接](https://example.com)\n";
    let template = MarkdownAdapter::build_template(text, false);
    let built = crate::textual_template::slots::build_slots(&template);
    let rebuilt = crate::textual_template::rebuild::rebuild_text(&template, &built.slots).unwrap();
    assert_eq!(rebuilt, text);
}
```

- [ ] **Step 2: 运行 Markdown 定向测试，确认当前适配器只有 `split_regions(...)` 主链而失败**

Run:

```bash
/mnt/c/Windows/System32/cmd.exe /C "cd /d E:\Code\LessAI\src-tauri && cargo test markdown_template -- --nocapture"
```

Expected: FAIL，报 `build_template` 不存在，或没有 `TextTemplate` 回构能力。

- [ ] **Step 3: 拆分 `markdown.rs` 并实现 `build_template(...)`**

```rust
// src-tauri/src/adapters/markdown/mod.rs
mod blocks;
mod inline;
mod template;
#[cfg(test)]
mod tests;

pub struct MarkdownAdapter;

impl MarkdownAdapter {
    pub fn build_template(text: &str, rewrite_headings: bool) -> TextTemplate {
        template::build_template(text, rewrite_headings)
    }
}
```

- [ ] **Step 4: 接通导入与 selection rewrite，让 Markdown 不再走临时 `TextRegion` 链**

```rust
let template = MarkdownAdapter::build_template(source_text, rewrite_headings);
let built = textual_template::slots::build_slots(&template);
let request = build_rewrite_unit_request_from_slots(SELECTION_REWRITE_UNIT_ID, &built.slots, format);
```

- [ ] **Step 5: 重跑 Markdown 定向测试与 selection 回归**

Run:

```bash
/mnt/c/Windows/System32/cmd.exe /C "cd /d E:\Code\LessAI\src-tauri && cargo test markdown_template -- --nocapture"
/mnt/c/Windows/System32/cmd.exe /C "cd /d E:\Code\LessAI\src-tauri && cargo test rewrite_selection -- --nocapture"
```

Expected: PASS，Markdown block/inline 锁定边界稳定，selection 只发一次请求且不越过锁定边界。

- [ ] **Step 6: 提交 Markdown 模板迁移**

```bash
git add src-tauri/src/adapters/markdown src-tauri/src/adapters/mod.rs src-tauri/src/documents/source.rs src-tauri/src/rewrite/llm/selection.rs
git commit -m "迁移markdown到模板化分块主链"
```

### Task 4: 将 TeX 迁移到模板主链，并拆分超长适配器文件

**Files:**
- Create: `src-tauri/src/adapters/tex/mod.rs`
- Create: `src-tauri/src/adapters/tex/blocks.rs`
- Create: `src-tauri/src/adapters/tex/commands.rs`
- Create: `src-tauri/src/adapters/tex/template.rs`
- Create: `src-tauri/src/adapters/tex/tests.rs`
- Delete: `src-tauri/src/adapters/tex.rs`
- Modify: `src-tauri/src/adapters/mod.rs`
- Modify: `src-tauri/src/documents/source.rs`
- Modify: `src-tauri/src/rewrite/llm/selection.rs`

- [ ] **Step 1: 写失败测试，钉死 TeX 的命令壳、环境壳、数学与注释边界**

```rust
#[test]
fn tex_template_locks_math_comments_and_command_shells() {
    let text = "\\section{标题}\n正文 $E=mc^2$ % 注释\n\\href{https://example.com}{链接文字}\n";
    let template = TexAdapter::build_template(text, false);
    let built = crate::textual_template::slots::build_slots(&template);

    assert!(built.slots.iter().any(|slot| slot.text == "$E=mc^2$" && !slot.editable));
    assert!(built.slots.iter().any(|slot| slot.text.contains("% 注释") && !slot.editable));
    assert!(built.slots.iter().any(|slot| slot.text == "\\href{" && !slot.editable));
    assert!(built.slots.iter().any(|slot| slot.text == "链接文字" && slot.editable));
}

#[test]
fn tex_template_rebuild_round_trips_original_text() {
    let text = "\\textbf{加粗文本}\n\n第二段。\n";
    let template = TexAdapter::build_template(text, false);
    let built = crate::textual_template::slots::build_slots(&template);
    let rebuilt = crate::textual_template::rebuild::rebuild_text(&template, &built.slots).unwrap();
    assert_eq!(rebuilt, text);
}
```

- [ ] **Step 2: 运行 TeX 定向测试，确认当前仍只有 `split_regions(...)` 生产入口而失败**

Run:

```bash
/mnt/c/Windows/System32/cmd.exe /C "cd /d E:\Code\LessAI\src-tauri && cargo test tex_template -- --nocapture"
```

Expected: FAIL，报 `build_template` 不存在或不能从模板重建原文。

- [ ] **Step 3: 拆分 TeX 适配器并实现 `build_template(...)`**

```rust
pub struct TexAdapter;

impl TexAdapter {
    pub fn build_template(text: &str, rewrite_headings: bool) -> TextTemplate {
        let blocks = blocks::scan_blocks(text, rewrite_headings);
        template::build_template(blocks)
    }
}
```

- [ ] **Step 4: 接通导入与 selection rewrite，让 TeX 也完全依赖模板+slot**

```rust
match format {
    DocumentFormat::Tex => {
        let template = TexAdapter::build_template(source_text, rewrite_headings);
        let built = textual_template::slots::build_slots(&template);
        build_rewrite_unit_request_from_slots(SELECTION_REWRITE_UNIT_ID, &built.slots, format)
    }
    _ => { /* 其他格式维持现有新主链 */ }
}
```

- [ ] **Step 5: 重跑 TeX 定向测试与 selection 回归**

Run:

```bash
/mnt/c/Windows/System32/cmd.exe /C "cd /d E:\Code\LessAI\src-tauri && cargo test tex_template -- --nocapture"
/mnt/c/Windows/System32/cmd.exe /C "cd /d E:\Code\LessAI\src-tauri && cargo test rewrite_selection -- --nocapture"
```

Expected: PASS，TeX 锁定边界稳定，selection 不会越过命令壳、数学和注释边界。

- [ ] **Step 6: 提交 TeX 模板迁移**

```bash
git add src-tauri/src/adapters/tex src-tauri/src/adapters/mod.rs src-tauri/src/documents/source.rs src-tauri/src/rewrite/llm/selection.rs
git commit -m "迁移tex到模板化分块主链"
```

### Task 5: 收口 `writeback / refresh / selection` 到模板+slot 真相，并删除旧生产链

**Files:**
- Modify: `src-tauri/src/documents/source.rs`
- Modify: `src-tauri/src/documents/writeback.rs`
- Modify: `src-tauri/src/documents.rs`
- Modify: `src-tauri/src/session_refresh.rs`
- Modify: `src-tauri/src/session_refresh/rules.rs`
- Modify: `src-tauri/src/session_refresh/draft.rs`
- Modify: `src-tauri/src/rewrite_writeback.rs`
- Modify: `src-tauri/src/rewrite/llm/selection.rs`
- Modify: `src-tauri/src/documents_writeback_tests.rs`
- Modify: `src-tauri/src/session_refresh/refresh_structure_tests.rs`
- Modify: `src-tauri/src/rewrite_writeback_tests.rs`
- Modify: `src-tauri/src/rewrite/llm_regression_tests.rs`

- [ ] **Step 1: 写失败测试，钉死“结构没变才允许写回”和“有脏状态时结构变化必须 Block”**

```rust
#[test]
fn refresh_blocks_dirty_markdown_session_when_template_signature_changes() {
    let mut existing = sample_session();
    existing.template_kind = Some("markdown".to_string());
    existing.template_signature = Some("old".to_string());
    existing.slot_structure_signature = Some("old-slots".to_string());
    existing.suggestions.push(rewrite_suggestion(
        "sg-1",
        1,
        "unit-0",
        "旧文本",
        "新文本",
        SuggestionDecision::Applied,
        vec![SlotUpdate::new("md:b1:r0:s0", "新文本")],
    ));

    let loaded = sample_markdown_loaded_source_with_signature("new", "new-slots");
    let refreshed = refresh_session_from_loaded(
        &existing,
        Path::new("/tmp/example.md"),
        loaded,
        SegmentationPreset::Paragraph,
        false,
        None,
    );

    assert_eq!(refreshed.session.status, RunningState::Failed);
}

#[test]
fn validate_textual_slot_writeback_rejects_boundary_drift() {
    let (root, path) = write_temp_file("boundary-drift", "md", "正文 `code`".as_bytes());
    let loaded = load_document_source(&path, false).expect("load md");
    let snapshot = capture_document_snapshot(&path).expect("snapshot");
    let mut updated = loaded.writeback_slots.clone();
    updated.retain(|slot| slot.editable);

    let error = execute_document_writeback(
        &path,
        &loaded.source_text,
        Some(&snapshot),
        DocumentWriteback::Slots(&updated),
        WritebackMode::Validate,
    )
    .expect_err("dropping locked slot must fail");

    assert!(error.contains("结构") || error.contains("locked"));
    cleanup_dir(&root);
}
```

- [ ] **Step 2: 运行收口相关测试，确认当前主链仍有旧 `TextRegion`/整篇文本覆盖残留**

Run:

```bash
/mnt/c/Windows/System32/cmd.exe /C "cd /d E:\Code\LessAI\src-tauri && cargo test boundary_drift -- --nocapture"
/mnt/c/Windows/System32/cmd.exe /C "cd /d E:\Code\LessAI\src-tauri && cargo test refresh_blocks_dirty_markdown_session_when_template_signature_changes -- --exact"
```

Expected: FAIL，前者会走文本覆盖或不做模板签名校验，后者不会基于模板签名给出正确的 `Rebuild / Block` 行为。

- [ ] **Step 3: 在写回链中统一使用模板校验与文本重建**

```rust
match writeback {
    DocumentWriteback::Slots(updated_slots) => {
        let template = source
            .template_snapshot
            .as_ref()
            .ok_or_else(|| "当前会话缺少模板快照，无法安全写回。".to_string())?;
        let validated = textual_template::validate::validate_slot_writeback(
            template,
            source.template_signature.as_deref(),
            source.slot_structure_signature.as_deref(),
            updated_slots,
        )?;
        let rebuilt = textual_template::rebuild::rebuild_text(template, &validated)?;
        Ok(normalize_text_against_source_layout(expected_source_text, &rebuilt).into_bytes())
    }
    DocumentWriteback::Text(updated_text) => build_text_writeback_bytes(source, expected_source_text, updated_text),
}
```

- [ ] **Step 4: 把 refresh 和 selection 全部收口到模板闭环，并删除旧生产出口**

```rust
let structure_changed =
    existing.template_signature != loaded.template_signature
        || existing.slot_structure_signature != loaded.slot_structure_signature
        || existing.segmentation_preset != Some(segmentation_preset)
        || existing.rewrite_headings != Some(rewrite_headings);

// documents.rs
pub(crate) use source::{document_format, document_session_id, is_docx_path, load_document_source, LoadedDocumentSource};
// 删除 writeback_slots_from_regions 的生产导出
```

- [ ] **Step 5: 运行全量后端回归与前端类型检查**

Run:

```bash
/mnt/c/Windows/System32/cmd.exe /C "cd /d E:\Code\LessAI\src-tauri && cargo test -- --nocapture"
pnpm run typecheck
```

Expected: PASS。`txt / md / tex / docx / pdf` 现有用例全部通过，TypeScript 不出现新增错误。

- [ ] **Step 6: 提交主链收口和旧链删除**

```bash
git add src-tauri/src/documents/source.rs src-tauri/src/documents/writeback.rs src-tauri/src/documents.rs src-tauri/src/session_refresh.rs src-tauri/src/session_refresh/rules.rs src-tauri/src/session_refresh/draft.rs src-tauri/src/rewrite_writeback.rs src-tauri/src/rewrite/llm/selection.rs src-tauri/src/documents_writeback_tests.rs src-tauri/src/session_refresh/refresh_structure_tests.rs src-tauri/src/rewrite_writeback_tests.rs src-tauri/src/rewrite/llm_regression_tests.rs
git commit -m "收口文本格式模板化写回主链"
```

### Task 6: 做严格清尾，确认旧链和死代码已经移除

**Files:**
- Modify: `src-tauri/src/documents_source_tests.rs`
- Modify: `src-tauri/src/documents_writeback_tests.rs`
- Modify: `src-tauri/src/rewrite/llm_regression_tests.rs`
- Modify: `src-tauri/src/rewrite_writeback_tests.rs`
- Modify: `src-tauri/src/session_refresh/refresh_structure_tests.rs`
- Modify: 其余被新主链代替的测试辅助和导出

- [ ] **Step 1: 搜索并删除旧生产链残留调用**

Run:

```bash
rg "writeback_slots_from_regions|plain_text_regions\\(|split_regions\\(" src-tauri/src
```

Expected: 只允许测试辅助或适配器内部局部 helper 命中；不允许 `documents/source.rs`、`documents/writeback.rs`、`rewrite/llm/selection.rs` 等生产主链再命中旧入口。

- [ ] **Step 2: 清理被新主链代替的死代码和测试旧断言**

```rust
// 删除此类旧导出
pub(crate) use source::writeback_slots_from_regions;

// 改成模板闭环断言
assert_eq!(loaded.template_kind.as_deref(), Some("markdown"));
assert_eq!(loaded.writeback_slots[0].anchor.as_deref(), Some("md:b0:r0:s0"));
```

- [ ] **Step 3: 做最终验证**

Run:

```bash
/mnt/c/Windows/System32/cmd.exe /C "cd /d E:\Code\LessAI\src-tauri && cargo test -- --nocapture"
pnpm run typecheck
git diff --check
```

Expected: 全部 PASS；`git diff --check` 没有新增空白错误。

- [ ] **Step 4: 提交最终清尾**

```bash
git add src-tauri/src
git commit -m "清理文本格式旧分块写回链"
```

## Self-Review

- **Spec coverage:** Task 1 覆盖共享模板模型和 session 持久化；Task 2、3、4 分别完成 `txt / markdown / tex` 的 anchored-slot 纵向闭环；Task 5 覆盖统一写回、刷新、selection 和旧链删除；Task 6 覆盖死代码、残留调用和全量验证。
- **Placeholder scan:** 计划中没有 `TODO`、`TBD`、`implement later`、`类似 Task N` 这类占位描述；每个任务都给了明确文件、代码片段、命令和预期结果。
- **Type consistency:** 计划统一使用 `TextTemplate`、`template_signature`、`slot_structure_signature`、`template_snapshot`、`WritebackSlot[]` 这组命名；没有再混回 `TextRegion` 作为生产主链真相。

Plan complete and saved to `docs/superpowers/plans/2026-04-18-textual-format-anchored-slot-closure.md`. Two execution options:

**1. Subagent-Driven (recommended)** - I dispatch a fresh subagent per task, review between tasks, fast iteration

**2. Inline Execution** - Execute tasks in this session using executing-plans, batch execution with checkpoints

**Which approach?**
