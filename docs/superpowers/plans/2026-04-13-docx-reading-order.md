# Docx Reading Order Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 让 docx 导入结果尽量接近 Word 的阅读顺序，同时保持可改写与可写回严格一致。

**Architecture:** 在现有 `DocxAdapter` 外部接口不变的前提下，拆出编号解析、填写线识别、浮动对象顺序重建三个辅助模块。导入和写回共用同一套占位与区域模型，避免导入支持和写回支持脱节。

**Tech Stack:** Rust, quick-xml, zip, cargo test

---

### Task 1: 建立失败测试

**Files:**
- Modify: `src-tauri/src/adapters/docx/tests.rs`

- [ ] 增加“下划线空白 run 导入为 `[填写线]`”测试。
- [ ] 增加“numbering.xml 编号前缀可见且锁定”测试。
- [ ] 增加“浮动文本框不再内联夹进正文句子”测试。
- [ ] 在 Windows 环境运行相关 `cargo test`，确认先失败。

### Task 2: 拆分辅助模块

**Files:**
- Create: `src-tauri/src/adapters/docx/numbering.rs`
- Create: `src-tauri/src/adapters/docx/import_order.rs`
- Modify: `src-tauri/src/adapters/docx.rs`
- Modify: `src-tauri/src/adapters/docx/simple.rs`

- [ ] 抽出编号解析逻辑，读取 `word/numbering.xml`。
- [ ] 抽出导入期阅读顺序辅助逻辑，避免继续堆在 `simple.rs`。
- [ ] 保持现有写回模板结构不变，新增逻辑只影响支持范围内的导入顺序与锁定区生成。

### Task 3: 最小实现并回归

**Files:**
- Modify: `src-tauri/src/adapters/docx/simple.rs`
- Modify: `src-tauri/src/adapters/docx/placeholders.rs`
- Test: `src-tauri/src/adapters/docx/tests.rs`

- [ ] 实现 `[填写线]` 占位识别。
- [ ] 实现自动编号锁定前缀渲染。
- [ ] 实现浮动文本框独立块重排。
- [ ] 跑定向测试与相关回归测试，确认导入/写回未失配。
