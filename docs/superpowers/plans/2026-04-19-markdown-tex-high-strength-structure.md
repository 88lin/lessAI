# Markdown/TeX High-Strength Structure Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 将 `markdown / tex` 提升到与 `docx` 同级的高强度结构化支持，确保导入、分块、建议、编辑器写回、选区改写和最终写回都统一到 `template -> anchored slots -> rewrite units -> slot writeback` 单一路径。

**Architecture:** 继续使用现有 `textual_template` 作为共享基础设施，但去掉 `TextRegion -> markdown_template/tex_template/textual_regions_template` 这条旧桥接链。`markdown` 和 `tex` 改成“块级扫描 + 行内/命令壳扫描 + 模板组装”的目录模块；`documents/source`、`session_refresh`、`documents/writeback`、`editor_writeback` 和 `rewrite/llm/selection` 全部改为消费模板快照和 slot 结构签名，不再允许 `markdown/tex` 通过整篇文本覆盖绕过 slot 闭环。

**Tech Stack:** Rust, Tauri, serde, 现有 `textual_template` / `rewrite_unit` 管线, React + TypeScript, pnpm, Windows `cargo.exe`

---

## Planned File Map

- Create: `src-tauri/src/adapters/markdown/mod.rs`
- Create: `src-tauri/src/adapters/markdown/blocks.rs`
- Create: `src-tauri/src/adapters/markdown/inline.rs`
- Create: `src-tauri/src/adapters/markdown/template.rs`
- Create: `src-tauri/src/adapters/markdown/tests.rs`
- Create: `src-tauri/src/adapters/tex/mod.rs`
- Create: `src-tauri/src/adapters/tex/blocks.rs`
- Create: `src-tauri/src/adapters/tex/commands.rs`
- Create: `src-tauri/src/adapters/tex/template.rs`
- Create: `src-tauri/src/adapters/tex/tests.rs`
- Modify: `src-tauri/src/adapters/mod.rs`
- Modify: `src-tauri/src/textual_template/factory.rs`
- Modify: `src-tauri/src/documents/source.rs`
- Modify: `src-tauri/src/documents/textual.rs`
- Modify: `src-tauri/src/documents/writeback.rs`
- Modify: `src-tauri/src/session_refresh.rs`
- Modify: `src-tauri/src/session_refresh/rules.rs`
- Modify: `src-tauri/src/rewrite/llm/selection.rs`
- Modify: `src-tauri/src/editor_writeback.rs`
- Modify: `src-tauri/src/editor_writeback_tests.rs`
- Modify: `src-tauri/src/commands/editor.rs`
- Modify: `src-tauri/src/documents_source_tests.rs`
- Modify: `src-tauri/src/documents_writeback_tests.rs`
- Modify: `src-tauri/src/session_refresh/refresh_structure_tests.rs`
- Modify: `src-tauri/src/rewrite/llm_regression_tests.rs`
- Modify: `src/app/hooks/useDocumentActions.ts`
- Modify: `src/lib/api.ts`
- Delete: `src-tauri/src/adapters/markdown.rs`
- Delete: `src-tauri/src/adapters/markdown_template.rs`
- Delete: `src-tauri/src/adapters/tex.rs`
- Delete: `src-tauri/src/adapters/tex_template.rs`
- Delete: `src-tauri/src/adapters/textual_regions_template.rs`

### Task 1: 拆出 Markdown 结构模块并钉死模板/anchor/locked region

**Files:**
- Create: `src-tauri/src/adapters/markdown/mod.rs`
- Create: `src-tauri/src/adapters/markdown/blocks.rs`
- Create: `src-tauri/src/adapters/markdown/inline.rs`
- Create: `src-tauri/src/adapters/markdown/template.rs`
- Create: `src-tauri/src/adapters/markdown/tests.rs`
- Modify: `src-tauri/src/adapters/mod.rs`
- Test: `src-tauri/src/adapters/markdown/tests.rs`

- [ ] **Step 1: 先写失败测试，固定 Markdown 模板输出必须带稳定块锚点和可见 locked 壳**

```rust
#[test]
fn build_template_marks_markdown_syntax_shells_as_locked_regions() {
    let template = super::MarkdownAdapter::build_template(
        "1. [标题](https://example.com)\n",
        false,
    );

    assert_eq!(template.kind, "markdown");
    assert_eq!(template.blocks.len(), 1);
    assert_eq!(template.blocks[0].anchor, "md:b0");
    assert_eq!(template.blocks[0].kind, "list_item");
    assert_eq!(
        template.blocks[0]
            .regions
            .iter()
            .map(|region| (region.anchor.as_str(), region.editable))
            .collect::<Vec<_>>(),
        vec![
            ("md:b0:r0", false),
            ("md:b0:r1", true),
            ("md:b0:r2", false),
        ]
    );
}

#[test]
fn build_template_locks_fenced_code_block_as_single_locked_block() {
    let template = super::MarkdownAdapter::build_template(
        "```rust\nfn main() {}\n```\n",
        false,
    );

    assert_eq!(template.blocks.len(), 1);
    assert_eq!(template.blocks[0].kind, "locked_block");
    assert!(template.blocks[0].regions.iter().all(|region| !region.editable));
}
```

- [ ] **Step 2: 运行定向测试，确认当前因为模块尚未拆分而失败**

Run:

```bash
/mnt/c/Windows/System32/cmd.exe /C "cd /d E:\Code\LessAI\src-tauri && cargo test build_template_marks_markdown_syntax_shells_as_locked_regions -- --exact"
/mnt/c/Windows/System32/cmd.exe /C "cd /d E:\Code\LessAI\src-tauri && cargo test build_template_locks_fenced_code_block_as_single_locked_block -- --exact"
```

Expected: FAIL，报 `src-tauri/src/adapters/markdown/tests.rs` 不存在，或 `MarkdownAdapter::build_template` / 新模块路径不匹配。

- [ ] **Step 3: 写最小实现，把 Markdown 拆成 blocks / inline / template 三层**

```rust
// src-tauri/src/adapters/markdown/mod.rs
mod blocks;
mod inline;
mod template;

pub struct MarkdownAdapter;

impl MarkdownAdapter {
    pub fn build_template(text: &str, rewrite_headings: bool) -> crate::textual_template::TextTemplate {
        template::build_template(text, rewrite_headings)
    }
}

#[cfg(test)]
mod tests;
```

```rust
// src-tauri/src/adapters/markdown/template.rs
use crate::textual_template::models::{TextTemplate, TextTemplateBlock, TextTemplateRegion};

pub(super) fn build_template(text: &str, rewrite_headings: bool) -> TextTemplate {
    let blocks = super::blocks::scan_blocks(text, rewrite_headings)
        .into_iter()
        .enumerate()
        .map(|(block_index, block)| TextTemplateBlock {
            anchor: format!("md:b{block_index}"),
            kind: block.kind,
            regions: super::inline::build_regions(block_index, block),
        })
        .collect::<Vec<_>>();

    TextTemplate::new("markdown", blocks)
}
```

- [ ] **Step 4: 运行 Markdown 模块测试和现有导入测试，确认新模板稳定**

Run:

```bash
/mnt/c/Windows/System32/cmd.exe /C "cd /d E:\Code\LessAI\src-tauri && cargo test adapters::markdown::tests -- --nocapture"
/mnt/c/Windows/System32/cmd.exe /C "cd /d E:\Code\LessAI\src-tauri && cargo test load_markdown_source_builds_template_metadata_and_anchors -- --exact"
```

Expected: PASS，且 `load_markdown_source_*` 断言中的锚点仍然稳定为 `md:b*:r*:s*`。

- [ ] **Step 5: 提交 Markdown 结构模块拆分**

```bash
git add src-tauri/src/adapters/markdown src-tauri/src/adapters/mod.rs src-tauri/src/documents_source_tests.rs
git commit -m "拆分markdown结构模板模块"
```

### Task 2: 拆出 TeX 结构模块并固定命令壳/环境壳的 locked region

**Files:**
- Create: `src-tauri/src/adapters/tex/mod.rs`
- Create: `src-tauri/src/adapters/tex/blocks.rs`
- Create: `src-tauri/src/adapters/tex/commands.rs`
- Create: `src-tauri/src/adapters/tex/template.rs`
- Create: `src-tauri/src/adapters/tex/tests.rs`
- Modify: `src-tauri/src/adapters/mod.rs`
- Test: `src-tauri/src/adapters/tex/tests.rs`

- [ ] **Step 1: 先写失败测试，固定 TeX 模板必须把命令壳和原样环境锁成不可改写区**

```rust
#[test]
fn build_template_keeps_tex_command_shell_locked_and_argument_editable() {
    let template = super::TexAdapter::build_template("\\textbf{重点}", false);

    assert_eq!(template.kind, "tex");
    assert_eq!(template.blocks.len(), 1);
    assert_eq!(template.blocks[0].anchor, "tex:b0");
    assert_eq!(template.blocks[0].kind, "command_block");
    assert_eq!(
        template.blocks[0]
            .regions
            .iter()
            .map(|region| (region.anchor.as_str(), region.editable))
            .collect::<Vec<_>>(),
        vec![
            ("tex:b0:r0", false),
            ("tex:b0:r1", true),
            ("tex:b0:r2", false),
        ]
    );
}

#[test]
fn build_template_locks_verbatim_environment_as_single_locked_block() {
    let template = super::TexAdapter::build_template(
        "\\begin{verbatim}\nraw\n\\end{verbatim}\n",
        false,
    );

    assert_eq!(template.blocks.len(), 1);
    assert_eq!(template.blocks[0].kind, "locked_block");
    assert!(template.blocks[0].regions.iter().all(|region| !region.editable));
}
```

- [ ] **Step 2: 运行定向测试，确认当前因为目录模块尚未建立而失败**

Run:

```bash
/mnt/c/Windows/System32/cmd.exe /C "cd /d E:\Code\LessAI\src-tauri && cargo test build_template_keeps_tex_command_shell_locked_and_argument_editable -- --exact"
/mnt/c/Windows/System32/cmd.exe /C "cd /d E:\Code\LessAI\src-tauri && cargo test build_template_locks_verbatim_environment_as_single_locked_block -- --exact"
```

Expected: FAIL，报 `src-tauri/src/adapters/tex/tests.rs` 不存在，或 `TexAdapter::build_template` / 模块路径不匹配。

- [ ] **Step 3: 写最小实现，把 TeX 拆成 blocks / commands / template 三层**

```rust
// src-tauri/src/adapters/tex/mod.rs
mod blocks;
mod commands;
mod template;

pub struct TexAdapter;

impl TexAdapter {
    pub fn build_template(text: &str, rewrite_headings: bool) -> crate::textual_template::TextTemplate {
        template::build_template(text, rewrite_headings)
    }
}

#[cfg(test)]
mod tests;
```

```rust
// src-tauri/src/adapters/tex/template.rs
use crate::textual_template::models::{TextTemplate, TextTemplateBlock};

pub(super) fn build_template(text: &str, rewrite_headings: bool) -> TextTemplate {
    let blocks = super::blocks::scan_blocks(text, rewrite_headings)
        .into_iter()
        .enumerate()
        .map(|(block_index, block)| TextTemplateBlock {
            anchor: format!("tex:b{block_index}"),
            kind: block.kind,
            regions: super::commands::build_regions(block_index, block),
        })
        .collect::<Vec<_>>();

    TextTemplate::new("tex", blocks)
}
```

- [ ] **Step 4: 运行 TeX 模块测试和现有导入测试**

Run:

```bash
/mnt/c/Windows/System32/cmd.exe /C "cd /d E:\Code\LessAI\src-tauri && cargo test adapters::tex::tests -- --nocapture"
/mnt/c/Windows/System32/cmd.exe /C "cd /d E:\Code\LessAI\src-tauri && cargo test load_tex_source_builds_template_metadata_and_anchors -- --exact"
```

Expected: PASS，且现有 `load_tex_source_*` 锚点断言仍保持 `tex:b*:r*:s*`。

- [ ] **Step 5: 提交 TeX 结构模块拆分**

```bash
git add src-tauri/src/adapters/tex src-tauri/src/adapters/mod.rs src-tauri/src/documents_source_tests.rs
git commit -m "拆分tex结构模板模块"
```

### Task 3: 删除 `TextRegion` 旧桥接层并让导入主链只消费模板快照

**Files:**
- Modify: `src-tauri/src/adapters/mod.rs`
- Modify: `src-tauri/src/textual_template/factory.rs`
- Modify: `src-tauri/src/documents/textual.rs`
- Modify: `src-tauri/src/documents/source.rs`
- Modify: `src-tauri/src/documents_source_tests.rs`
- Delete: `src-tauri/src/adapters/markdown_template.rs`
- Delete: `src-tauri/src/adapters/tex_template.rs`
- Delete: `src-tauri/src/adapters/textual_regions_template.rs`

- [ ] **Step 1: 先写失败测试，固定导入结果必须带模板快照并保留 locked slot**

```rust
#[test]
fn load_markdown_source_persists_template_snapshot_and_locked_slots() {
    let (root, path) = write_temp_file(
        "markdown-template-snapshot",
        "md",
        "1. [标题](https://example.com)\n".as_bytes(),
    );

    let loaded = load_document_source(&path, false).expect("load markdown");

    assert_eq!(loaded.template_kind.as_deref(), Some("markdown"));
    assert!(loaded.template_snapshot.is_some());
    assert!(loaded.writeback_slots.iter().any(|slot| !slot.editable));
    cleanup_dir(&root);
}

#[test]
fn load_tex_source_persists_template_snapshot_and_locked_slots() {
    let (root, path) = write_temp_file(
        "tex-template-snapshot",
        "tex",
        "\\textbf{重点}".as_bytes(),
    );

    let loaded = load_document_source(&path, false).expect("load tex");

    assert_eq!(loaded.template_kind.as_deref(), Some("tex"));
    assert!(loaded.template_snapshot.is_some());
    assert!(loaded.writeback_slots.iter().any(|slot| !slot.editable));
    cleanup_dir(&root);
}
```

- [ ] **Step 2: 运行导入测试，确认当前旧桥接层仍在参与构造**

Run:

```bash
/mnt/c/Windows/System32/cmd.exe /C "cd /d E:\Code\LessAI\src-tauri && cargo test load_markdown_source_persists_template_snapshot_and_locked_slots -- --exact"
/mnt/c/Windows/System32/cmd.exe /C "cd /d E:\Code\LessAI\src-tauri && cargo test load_tex_source_persists_template_snapshot_and_locked_slots -- --exact"
```

Expected: 至少一条 FAIL，暴露当前导入仍有旧模板桥接或快照字段没有完全从格式模板直接产生。

- [ ] **Step 3: 写最小实现，删掉 `TextRegion` 主真相并把导入主链改成 `adapter -> template -> slots`**

```rust
// src-tauri/src/adapters/mod.rs
pub mod docx;
pub mod markdown;
pub mod pdf;
pub mod plain_text;
pub mod tex;
```

```rust
// src-tauri/src/textual_template/factory.rs
pub(crate) fn build_template(
    source_text: &str,
    format: DocumentFormat,
    rewrite_headings: bool,
) -> TextTemplate {
    match format {
        DocumentFormat::PlainText => crate::adapters::plain_text::PlainTextAdapter::build_template(source_text),
        DocumentFormat::Markdown => crate::adapters::markdown::MarkdownAdapter::build_template(source_text, rewrite_headings),
        DocumentFormat::Tex => crate::adapters::tex::TexAdapter::build_template(source_text, rewrite_headings),
    }
}
```

- [ ] **Step 4: 删除旧桥接文件，重新跑导入测试**

Run:

```bash
/mnt/c/Windows/System32/cmd.exe /C "cd /d E:\Code\LessAI\src-tauri && cargo test load_markdown_source_ -- --nocapture"
/mnt/c/Windows/System32/cmd.exe /C "cd /d E:\Code\LessAI\src-tauri && cargo test load_tex_source_ -- --nocapture"
```

Expected: PASS，且不再引用 `TextRegion`、`markdown_template.rs`、`tex_template.rs`、`textual_regions_template.rs`。

- [ ] **Step 5: 提交导入主链去桥接**

```bash
git add src-tauri/src/adapters/mod.rs src-tauri/src/textual_template/factory.rs src-tauri/src/documents/textual.rs src-tauri/src/documents/source.rs src-tauri/src/documents_source_tests.rs
git rm src-tauri/src/adapters/markdown_template.rs src-tauri/src/adapters/tex_template.rs src-tauri/src/adapters/textual_regions_template.rs
git commit -m "删除文本格式旧桥接链路"
```

### Task 4: 把 `markdown/tex` 写回与刷新收口到模板签名和 slot 结构签名

**Files:**
- Modify: `src-tauri/src/documents/writeback.rs`
- Modify: `src-tauri/src/session_refresh.rs`
- Modify: `src-tauri/src/session_refresh/rules.rs`
- Modify: `src-tauri/src/documents_writeback_tests.rs`
- Modify: `src-tauri/src/session_refresh/refresh_structure_tests.rs`

- [ ] **Step 1: 先写失败测试，固定 `markdown/tex` 结构变化时必须被显式阻断**

```rust
#[test]
fn validate_document_writeback_rejects_markdown_slot_update_crossing_locked_boundary() {
    let (root, target) = write_temp_file(
        "markdown-cross-locked",
        "md",
        "1. [标题](https://example.com)\n".as_bytes(),
    );
    let loaded = load_document_source(&target, false).expect("load markdown");
    let snapshot = capture_document_snapshot(&target).expect("snapshot");
    let mut slots = loaded.writeback_slots.clone();
    slots[0].text = "改坏前缀".to_string();

    let error = execute_document_writeback(
        &target,
        textual_writeback_context(&loaded, &snapshot),
        DocumentWriteback::Slots(&slots),
        WritebackMode::Validate,
    )
    .expect_err("locked slot mutation must fail");

    assert!(error.contains("locked") || error.contains("不可编辑") || error.contains("结构"));
    cleanup_dir(&root);
}

#[test]
fn refresh_blocks_dirty_markdown_session_when_template_signature_changes() {
    let mut existing = sample_session();
    existing.template_kind = Some("markdown".to_string());
    existing.template_signature = Some("old-template".to_string());
    existing.slot_structure_signature = Some("old-structure".to_string());
    existing.suggestions.push(crate::test_support::rewrite_suggestion(
        "suggestion-1",
        1,
        "unit-0",
        "原文",
        "改写后",
        crate::models::SuggestionDecision::Proposed,
        vec![],
    ));

    let loaded = LoadedDocumentSource {
        source_text: "第一段".to_string(),
        template_kind: Some("markdown".to_string()),
        template_signature: Some("new-template".to_string()),
        slot_structure_signature: Some("new-structure".to_string()),
        template_snapshot: Some(crate::textual_template::models::TextTemplate::single_paragraph("markdown", "md:b0", "第一段")),
        writeback_slots: vec![editable_slot("md:b0:r0:s0", 0, "第一段")],
        write_back_supported: true,
        write_back_block_reason: None,
        plain_text_editor_safe: true,
        plain_text_editor_block_reason: None,
    };

    let refreshed = refresh_session_from_loaded(
        &existing,
        Path::new("/tmp/example.md"),
        loaded,
        SegmentationPreset::Paragraph,
        false,
        None,
    );

    assert!(!refreshed.session.write_back_supported);
}
```

- [ ] **Step 2: 运行定向测试，确认当前至少一条仍错误放行或未以签名为准**

Run:

```bash
/mnt/c/Windows/System32/cmd.exe /C "cd /d E:\Code\LessAI\src-tauri && cargo test validate_document_writeback_rejects_markdown_slot_update_crossing_locked_boundary -- --exact"
/mnt/c/Windows/System32/cmd.exe /C "cd /d E:\Code\LessAI\src-tauri && cargo test refresh_blocks_dirty_markdown_session_when_template_signature_changes -- --exact"
```

Expected: FAIL，暴露当前 `markdown/tex` 写回或 refresh 仍有旧结构路径没有收紧。

- [ ] **Step 3: 写最小实现，让 `documents/writeback` 和 `session_refresh` 都只认模板签名与 slot 结构**

```rust
// src-tauri/src/documents/writeback.rs
fn build_slot_writeback_bytes(
    source: &VerifiedWritebackSource,
    context: DocumentWritebackContext<'_>,
    updated_slots: &[WritebackSlot],
) -> Result<Vec<u8>, String> {
    match source {
        VerifiedWritebackSource::Textual(template) => {
            textual_template::validate::ensure_template_signature(
                context.expected_template_signature,
                template,
            )?;
            textual_template::validate::ensure_slot_structure_signature(
                context.expected_slot_structure_signature,
                updated_slots,
            )?;
            let rebuilt = textual_template::rebuild::rebuild_text(template, updated_slots)?;
            Ok(normalize_text_against_source_layout(context.expected_source_text, &rebuilt).into_bytes())
        }
        VerifiedWritebackSource::Docx(current_bytes) => adapters::docx::DocxAdapter::write_updated_slots(
            current_bytes,
            context.expected_source_text,
            updated_slots,
        ),
    }
}
```

- [ ] **Step 4: 跑写回和 refresh 结构测试**

Run:

```bash
/mnt/c/Windows/System32/cmd.exe /C "cd /d E:\Code\LessAI\src-tauri && cargo test documents_writeback_tests -- --nocapture"
/mnt/c/Windows/System32/cmd.exe /C "cd /d E:\Code\LessAI\src-tauri && cargo test session_refresh::refresh_structure_tests -- --nocapture"
```

Expected: PASS，且失败原因明确落在模板签名、slot 结构或 locked 边界。

- [ ] **Step 5: 提交写回/刷新闭环收口**

```bash
git add src-tauri/src/documents/writeback.rs src-tauri/src/session_refresh.rs src-tauri/src/session_refresh/rules.rs src-tauri/src/documents_writeback_tests.rs src-tauri/src/session_refresh/refresh_structure_tests.rs
git commit -m "收口markdown和tex写回刷新闭环"
```

### Task 5: 让 `markdown/tex` 编辑器也走 slot edits，而不是整篇文本写回

**Files:**
- Modify: `src-tauri/src/editor_writeback.rs`
- Modify: `src-tauri/src/editor_writeback_tests.rs`
- Modify: `src-tauri/src/commands/editor.rs`
- Modify: `src/app/hooks/useDocumentActions.ts`
- Modify: `src/lib/api.ts`

- [ ] **Step 1: 先写失败测试，固定 `markdown/tex` 必须按槽位保存**

```rust
#[test]
fn build_slot_editor_writeback_allows_markdown_session() {
    let mut session = sample_text_session();
    session.document_path = "/tmp/example.md".to_string();
    session.template_kind = Some("markdown".to_string());
    session.writeback_slots = vec![
        editable_slot("md:b0:r0:s0", 0, "正文"),
        locked_slot("md:b0:r1:s0", 1, "]"),
    ];
    let edits = vec![EditorSlotEdit {
        slot_id: "md:b0:r0:s0".to_string(),
        text: "改写正文".to_string(),
    }];

    let payload = build_slot_editor_writeback(&session, &edits).expect("markdown slot writeback");
    assert!(matches!(payload, EditorWritebackPayload::Slots(_)));
}

#[test]
fn build_plain_text_editor_writeback_rejects_markdown_session() {
    let mut session = sample_text_session();
    session.document_path = "/tmp/example.md".to_string();
    session.template_kind = Some("markdown".to_string());

    let error = build_plain_text_editor_writeback(&session, "整篇覆盖")
        .expect_err("markdown text payload should be rejected");

    assert!(error.contains("按槽位保存"));
}
```

- [ ] **Step 2: 运行定向测试，确认当前后端仍只允许 docx 走 slot 编辑**

Run:

```bash
/mnt/c/Windows/System32/cmd.exe /C "cd /d E:\Code\LessAI\src-tauri && cargo test build_slot_editor_writeback_allows_markdown_session -- --exact"
/mnt/c/Windows/System32/cmd.exe /C "cd /d E:\Code\LessAI\src-tauri && cargo test build_plain_text_editor_writeback_rejects_markdown_session -- --exact"
```

Expected: FAIL，当前会报“当前仅 docx 支持按槽位编辑写回”或错误放行 Text payload。

- [ ] **Step 3: 写最小实现，让结构化文本格式统一走 slot edits**

```rust
// src-tauri/src/editor_writeback.rs
fn session_requires_slot_editor(session: &DocumentSession) -> bool {
    matches!(session.template_kind.as_deref(), Some("markdown" | "tex")) || is_docx_path(Path::new(&session.document_path))
}

pub(crate) fn build_plain_text_editor_writeback(
    session: &DocumentSession,
    content: &str,
) -> Result<EditorWritebackPayload, String> {
    if session_requires_slot_editor(session) {
        return Err("当前结构化文档必须按槽位保存，不能再走整篇纯文本写回。".to_string());
    }
    // 其余 plain text 逻辑保持
}

pub(crate) fn build_slot_editor_writeback(
    session: &DocumentSession,
    edits: &[EditorSlotEdit],
) -> Result<EditorWritebackPayload, String> {
    if !session_requires_slot_editor(session) {
        return Err("当前文档不需要按槽位编辑写回。".to_string());
    }
    // 现有 collect_slot_edit_updates + apply_slot_updates 逻辑保持
}
```

- [ ] **Step 4: 切前端保存入口，让 `markdown/tex` 和 `docx` 一样发送 `slotEdits`**

```tsx
// src/app/hooks/useDocumentActions.ts
const structuredEditor = isDocxPath(session.documentPath)
  || session.templateKind === "markdown"
  || session.templateKind === "tex";

const input = structuredEditor
  ? { kind: "slotEdits" as const, edits: buildEditorSlotEdits(session, editorSlotOverridesRef.current) }
  : { kind: "text" as const, content };

return runDocumentWriteback(session.id, "write", input, editorBaseSnapshotRef.current);
```

- [ ] **Step 5: 运行后端测试和前端类型检查**

Run:

```bash
/mnt/c/Windows/System32/cmd.exe /C "cd /d E:\Code\LessAI\src-tauri && cargo test editor_writeback_tests -- --nocapture"
pnpm run typecheck
```

Expected: PASS，前端 `EditorWritebackInput` 仍只保留 `text | slotEdits` 两种 shape，但 `markdown/tex` 保存路径改走 `slotEdits`。

- [ ] **Step 6: 提交编辑器写回主链收口**

```bash
git add src-tauri/src/editor_writeback.rs src-tauri/src/editor_writeback_tests.rs src-tauri/src/commands/editor.rs src/app/hooks/useDocumentActions.ts src/lib/api.ts
git commit -m "让结构化文本编辑器按槽位写回"
```

### Task 6: 切 `selection rewrite` 到模板/slot 主链并做全链验证

**Files:**
- Modify: `src-tauri/src/rewrite/llm/selection.rs`
- Modify: `src-tauri/src/rewrite/llm_regression_tests.rs`
- Modify: `src-tauri/src/rewrite_writeback_tests.rs`

- [ ] **Step 1: 先写失败测试，固定选区改写必须保留语法壳并只更新 editable slot**

```rust
#[test]
fn markdown_selection_rewrite_keeps_link_shell_locked() {
    let server = TestServer::start(vec![json_http_response(
        r#"{"rewriteUnitId":"selection","updates":[{"slotId":"md:b0:r1:s0","text":"新标题"}]}"#,
    )]);
    let settings = test_settings(&server.base_url);
    let client = build_client(&settings).unwrap();

    let result = run_async(rewrite_selection_text_with_client(
        &client,
        &settings,
        "[标题](https://example.com)",
        DocumentFormat::Markdown,
        false,
    ))
    .expect("selection rewrite should succeed");

    assert_eq!(result, "[新标题](https://example.com)");
    assert_eq!(server.request_count(), 1);
}
```

- [ ] **Step 2: 运行定向测试，确认当前 `selection rewrite` 仍走 `build_slots` 的旧捷径**

Run:

```bash
/mnt/c/Windows/System32/cmd.exe /C "cd /d E:\Code\LessAI\src-tauri && cargo test markdown_selection_rewrite_keeps_link_shell_locked -- --exact"
```

Expected: FAIL，当前选区改写会因为 slot id、模板来源或 locked 壳处理不一致而失败。

- [ ] **Step 3: 写最小实现，让选区改写先建模板，再建 slots，再发起单次模型调用**

```rust
// src-tauri/src/rewrite/llm/selection.rs
let template = crate::textual_template::factory::build_template(
    source_text,
    format,
    rewrite_headings,
);
let built = crate::textual_template::slots::build_slots(&template);
let slots = built.slots;
let request = build_rewrite_unit_request_from_slots(SELECTION_REWRITE_UNIT_ID, &slots, format);
```

- [ ] **Step 4: 跑选区回归和最终写回回归**

Run:

```bash
/mnt/c/Windows/System32/cmd.exe /C "cd /d E:\Code\LessAI\src-tauri && cargo test rewrite::llm_regression_tests -- --nocapture"
/mnt/c/Windows/System32/cmd.exe /C "cd /d E:\Code\LessAI\src-tauri && cargo test rewrite_writeback_tests -- --nocapture"
```

Expected: PASS，且 Markdown/TeX 选区改写不会破坏 locked 壳，仍是一条 selection batch = 一次模型调用。

- [ ] **Step 5: 做全链验证并提交**

Run:

```bash
/mnt/c/Windows/System32/cmd.exe /C "cd /d E:\Code\LessAI\src-tauri && cargo test"
pnpm run typecheck
node scripts/ui-regression.test.mjs
```

Expected: PASS；如果任何回归失败，先修回归再提交。

```bash
git add src-tauri/src/rewrite/llm/selection.rs src-tauri/src/rewrite/llm_regression_tests.rs src-tauri/src/rewrite_writeback_tests.rs
git commit -m "统一结构化文本选区改写链路"
```

## Self-Review

- Spec coverage:
  - 模板、anchor、locked region：Task 1-2
  - 删除旧桥接层：Task 3
  - 写回和 refresh 闭环：Task 4
  - 编辑器不再整篇覆盖 markdown/tex：Task 5
  - selection rewrite 与全链验证：Task 6
- Placeholder scan:
  - 未保留模糊待定项、延后项、或引用前一任务代替细节的写法
- Type consistency:
  - 计划统一使用 `template_kind`、`template_signature`、`slot_structure_signature`、`slotEdits`、`slot_updates`、`rewrite_units` 这套现有命名，没有引入第二套术语
