# Textual Format Anchored Slot Closure Design

## Goal

将 `txt / md / tex` 提升到与 `docx` 同一档的闭环能力：

- 分块基于稳定结构，而不是临时字符串切片
- `1 个用户块 = 1 个 rewrite unit`
- `1 个 batch = 1 次模型调用`
- suggestion 只通过 `SlotUpdate[]` 更新
- 写回前必须做结构一致性校验
- 能改写的内容必须能安全写回；不能安全写回时显式失败

本设计不包含 PDF 原位写回。PDF 仍属于“抽取文本后参与阅读与改写，但不支持安全覆盖写回”的独立问题。

## Confirmed Constraints

- 不增加 silent fallback、兼容兜底或模糊对齐。
- 不扩充 Markdown/TeX 新语法支持范围；仅把当前已支持结构迁移到统一闭环主链。
- 前端主消费模型保持 `DocumentSession -> writebackSlots -> rewriteUnits` 不变，尽量不引入大规模 UI 重写。
- `docx` 现有主链作为参照系，不在本阶段重构其写回模型。
- 结构变化时，系统应明确阻断继续写回，而不是尝试猜测性迁移。

## Problem

当前所有格式虽然已经统一到 `WritebackSlot[] -> RewriteUnit[]` 这条后半段主链，但只有 `docx` 具备完整的结构闭环：

- `docx` 先从 XML 解析出 `WritebackBlockTemplate / WritebackParagraphTemplate / WritebackRegionTemplate`
- 再生成带稳定 `anchor` 的 `WritebackSlot`
- 写回时按 `slot` 回映射到原结构，并在写回前做结构一致性校验

`txt / md / tex` 目前只共享了后半段分块器，没有共享前半段“结构模板 + 锚点 + slot 级写回”能力：

- `txt` 仍是纯文本直接变成 `TextRegion`
- `md`、`tex` 虽然能识别 locked/editable 区域，但输出仍是扁平 `TextRegion`
- 文本格式写回仍退化成整篇字符串覆盖，而不是 slot 级写回
- refresh 仍基于结果比对，而不是模板签名和 slot 结构签名

结果是：

- slot 在文本格式中更像临时切片，不是稳定写回单元
- “可改写”和“可安全写回”没有做到和 `docx` 一样严格的同源闭环
- selection rewrite、session refresh、最终写回之间仍存在潜在漂移

## Design Overview

将 `txt / md / tex` 统一改造成与 `docx` 同构的四段主链：

1. `FormatTemplate`
   各格式先生成结构模板，而不是直接产出最终字符串 slot
2. `AnchoredSlot`
   模板统一转换成带稳定锚点的 `WritebackSlot`
3. `RewriteUnit`
   统一分块器在 slot 流上生成用户可见块
4. `SlotWriteback`
   写回只接受 slot 级更新；写回前必须校验模板和 slot 结构仍一致

这条链的唯一真相是：

- `source_text`：源文件文本快照
- `template_snapshot`：格式结构快照
- `writeback_slots`：最小写回单元真相
- `rewrite_units`：slot 的组合视图，不再承载结构真相

## Unified Template Model

为 `txt / md / tex` 新增共享的文本模板基础设施，统一定义为：

- `TextTemplate`
  - `kind`
  - `blocks`
  - `template_signature`
- `TextTemplateBlock`
  - `anchor`
  - `kind`
  - `regions`
  - `separator_after`
- `TextTemplateRegion`
  - `anchor`
  - `text`
  - `editable`
  - `role`
  - `presentation`
  - `separator_after`

统一约束：

- anchor 不能基于显示文本内容计算
- anchor 必须来自“格式语法下的结构路径 + 顺序位置”
- 同一份源文件在无结构变化时重新导入，anchor 必须稳定
- 结构变化导致 anchor 无法稳定重建时，必须报结构不一致错误

## Format-Specific Templates

### `txt`

`txt` 采用最轻量模板：

- 块级：按段落分块
- 区域级：段内正文区域
- 原子细分：段内再按统一 clause 边界拆为可写回 slot

anchor 规则：

- 段落块：`txt:p{paragraph_index}`
- 段内 region：`txt:p{paragraph_index}:r{region_index}`
- 原子分裂：`txt:p{paragraph_index}:r{region_index}:s{split_index}`

### `markdown`

Markdown 模板分为块级与块内两层：

- `ParagraphBlock`
- `HeadingBlock`
- `QuoteBlock`
- `ListItemBlock`
- `LockedBlock`
  - fenced code
  - table
  - front matter
  - reference definition
  - html block

块内 region 再区分：

- `EditableRegion`
- `LockedRegion`
  - inline code
  - link syntax
  - footnote / citation
  - bare URL
  - inline math
  - inline html

anchor 规则：

- 块：`md:b{block_index}`
- region：`md:b{block_index}:r{region_index}`
- 原子分裂：`md:b{block_index}:r{region_index}:s{split_index}`

### `tex`

TeX 模板同样分块级与块内两层，但块更偏语法结构：

- `ParagraphBlock`
- `CommandBlock`
- `EnvironmentBlock`
- `MathBlock`
- `LockedBlock`

块内 region 再区分：

- `EditableRegion`
- `LockedRegion`
  - command shell
  - environment shell
  - math
  - comment
  - raw content

anchor 规则：

- 块：`tex:b{block_index}`
- region：`tex:b{block_index}:r{region_index}`
- 原子分裂：`tex:b{block_index}:r{region_index}:s{split_index}`

## Session and Signature Model

`LoadedDocumentSource` 与 `DocumentSession` 需要新增结构快照字段：

- `template_kind`
- `template_signature`
- `slot_structure_signature`
- `template_snapshot`

职责区分：

- `template_signature`
  校验块级与 region 级骨架是否仍一致
- `slot_structure_signature`
  校验 slot 的顺序、anchor、editable/locked 边界、separator 结构是否仍一致
- `template_snapshot`
  用于 slot 级写回时按原模板重建最终文本

## Unified Writeback and Refresh Closure

写回前统一执行以下顺序：

1. 重新加载源文件
2. 重新解析模板
3. 重新生成 anchored slots
4. 校验 `template_signature`
5. 校验 `slot_structure_signature`
6. 校验 `SlotUpdate[]` 仅更新当前 unit / batch 内的 editable slot
7. 通过模板 + updated slots 重建最终文本
8. validate 模式仅校验；write 模式原子落盘

统一刷新逻辑：

- 模板与 slot 结构都一致：`Keep`
- 结构需重建但会话干净：`Rebuild`
- 存在 suggestion / 活动任务且结构变化：`Block`

不做模糊迁移或兼容回退。

## Runtime Flow

导入主链：

1. adapter 产出 `TextTemplate`
2. `textual_template::build_slots(...)`
3. `build_rewrite_units(...)`
4. `session_builder` 持久化 template 快照与签名

改写主链：

1. `RewriteUnit` 基于 slot 流形成真实用户块
2. `units_per_batch` 决定一次请求承载多少个 unit
3. LLM 只返回 `SlotUpdate[]`
4. suggestion 只保存 `slot_updates`

写回主链：

1. `rewrite_writeback` 构建 applied slot projection
2. 文本格式不再走整篇 `Text` 覆盖
3. 统一走 `Slots` 写回

selection rewrite 主链：

- 选区文本也通过对应格式的 template builder 生成 selection-scoped slots
- 不再走临时 `TextRegion` 切块链

## Module Changes

新增共享模块：

- `src-tauri/src/textual_template/mod.rs`
- `src-tauri/src/textual_template/models.rs`
- `src-tauri/src/textual_template/signature.rs`
- `src-tauri/src/textual_template/slots.rs`
- `src-tauri/src/textual_template/rebuild.rs`
- `src-tauri/src/textual_template/validate.rs`

新增或替换格式入口：

- `src-tauri/src/adapters/plain.rs`
- `src-tauri/src/adapters/markdown.rs`：主入口改为 `build_template(...)`
- `src-tauri/src/adapters/tex.rs`：主入口改为 `build_template(...)`

需要切主链的公共模块：

- `src-tauri/src/documents/source.rs`
- `src-tauri/src/documents/writeback.rs`
- `src-tauri/src/session_builder.rs`
- `src-tauri/src/session_refresh.rs`
- `src-tauri/src/rewrite_writeback.rs`
- `src-tauri/src/rewrite/llm/selection.rs`

## Migration Plan

按六阶段推进：

1. 先搭 `textual_template` 基础设施与最小闭环测试
2. 先切 `txt`，验证模板、anchored slot、slot 写回、结构签名校验
3. 再切 `markdown`，把现有结构识别迁移到模板主链
4. 再切 `tex`，把现有 locked/editable 识别迁移到模板主链
5. 统一 refresh、selection rewrite、final writeback 主链
6. 删除旧的 `TextRegion -> writeback_slots_from_regions` 生产主链与文本整篇覆盖写回

## Validation

完成标准分五组：

### 1. 模板与 slot 闭环

- `source_text -> template -> slots -> rebuild(source)` 与原文一致
- slot 的 `id / anchor / editable / separator_after` 稳定

### 2. rewrite unit 与 batch 语义

- `paragraph / sentence / clause` 得到预期 unit 数
- `1 个 unit = 1 个用户块`
- `1 个 batch = 1 次模型调用`
- `SlotUpdate[]` 不得越出 unit / batch 边界

### 3. 写回闭环

- editable slot 可 validate / write
- locked slot 更新被拒绝
- 越界更新被拒绝
- 写回后重新导入仍结构一致

### 4. refresh 与外部变化

- 外部文本变化但结构未变：可 clean rebuild
- 外部结构变化：显式 block
- suggestion / 活动任务存在时遇结构变化：阻断继续写回

### 5. 端到端回归

- 导入
- session 构建
- unit 构建
- slot update 应用
- 最终写回
- reload / refresh

## Out of Scope

- PDF 原位写回
- Markdown 新语法扩展
- TeX 新命令 / 新环境扩展
- 前端交互层重做
- docx 主链重构
