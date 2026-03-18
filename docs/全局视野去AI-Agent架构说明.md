# LessAI 全局视野去 AI Agent 架构说明

## 1. 产品定位重构

当前版本的 LessAI 更接近一个 `人控的分段改写工作台`：

- 输入文章
- 自动切段
- 逐段调用模型
- 查看 diff
- 手工接受 / 拒绝 / 重试
- 拼接最终结果

这个方向没有错，但它的核心能力仍然是 `chunk-centric`，也就是围绕单段处理。

如果产品目标升级为 `具有全局视野的文章 / 论文去 AI 化 Agent`，系统的主语就必须从“片段”切换为“文档”：

- 先理解整篇文章，而不是先切段
- 先抽取整体结构，再决定局部如何改
- 局部改写必须服从全文策略
- 最终结果必须经过文档级一致性检查
- 用户看到的也不该只是当前片段，而应当看到全文地图、风险点和改写计划


## 2. 北极星能力

目标 Agent 至少要具备以下 6 个能力：

### 2.1 文档级理解

- 识别标题、摘要、正文、结论、参考文献等结构
- 识别章节关系和段落层级
- 抽取文章主题、中心论点、关键术语、专有名词
- 识别必须保护的内容
  - 公式
  - 引用
  - 数据
  - 文献标号
  - 固定术语

### 2.2 全局改写规划

- 决定哪些章节需要优先处理
- 决定哪些段落需要强改、轻改或保护不改
- 决定不同区域的改写强度
- 形成统一的写作风格约束

### 2.3 带上下文的局部执行

- 每次改写某个片段时，不只传当前段
- 同时带上：
  - 文档摘要
  - 当前章节摘要
  - 邻近段落摘要
  - 全局术语表
  - 风格约束
  - 禁改规则

### 2.4 文档级复核

- 检查术语是否统一
- 检查风格是否飘移
- 检查是否破坏逻辑衔接
- 检查引用和数据是否被误改
- 检查不同章节之间是否出现重复或断裂

### 2.5 人类监管

- 用户可在文档级看整体状态
- 用户可定位高风险片段
- 用户可接受 / 拒绝 / 回滚某一段或某一批段
- 用户可查看 Agent 为什么这样改

### 2.6 可追溯性

- 每段改写要有来源上下文
- 每轮改写要有策略版本
- 全局规划、局部执行、复核结论都要能回查


## 3. 当前版本与目标版本的差距

### 3.1 当前已有能力

- 本地会话管理
- API 配置和测试连接
- 纯文本导入
- 按长度分段
- 逐段改写
- diff 高亮
- 手动审阅
- 自动串行处理
- 最终文本导出

### 3.2 当前缺失的关键能力

- 没有全文摘要和章节树
- 没有术语表和禁改规则
- 没有文档级改写计划
- 没有带上下文的局部执行
- 没有二次全局统一检查
- 没有风险分级和问题面板
- 没有论文结构专用处理
- 没有解释层，用户看不到 agent 的决策依据


## 4. 目标系统分层

建议把系统拆成 7 层。

### 4.1 文档接入层

职责：

- 导入 TXT / MD
- 后续扩展 DOCX / PDF
- 统一做文本标准化
- 切出基础结构块

输出：

- `RawDocument`
- `NormalizedDocument`

### 4.2 文档分析层

职责：

- 识别章节结构
- 提炼全文摘要
- 提炼各章节摘要
- 提取术语、实体、引用模式、数字事实
- 标注高风险区域

输出：

- `DocumentBrief`
- `SectionTree`
- `TermGlossary`
- `ProtectionRules`
- `RiskMap`

### 4.3 规划层

职责：

- 生成文档级改写计划
- 为每个 section 分配改写策略
- 为每个 chunk 分配优先级和约束
- 决定执行顺序

输出：

- `RewritePlan`
- `SectionPlan`
- `ChunkPlan`

### 4.4 执行层

职责：

- 根据计划逐段改写
- 每次带入全文上下文和局部上下文
- 产出候选稿、局部 diff、改写解释

输出：

- `ChunkRewriteResult`

### 4.5 文档统一层

职责：

- 在全部或局部改写后，做术语统一、风格统一、逻辑衔接检查
- 生成全局问题清单
- 必要时触发二次修正

输出：

- `GlobalConsistencyReport`
- `GlobalIssues`

### 4.6 人工审阅层

职责：

- 文档级总览
- 片段级 diff
- 批量接受 / 拒绝 / 回滚
- 跳转到风险段

输出：

- `ReviewDecisions`

### 4.7 持久化与追踪层

职责：

- 会话持久化
- 版本记录
- 规划记录
- 执行日志
- 复核日志


## 5. 必须新增的数据模型

当前只有 `DocumentSession / ChunkTask` 不够。

建议新增以下核心模型。

### 5.1 文档级模型

#### `DocumentBrief`

- `document_id`
- `title`
- `document_type`
  - article
  - paper
  - report
- `global_summary`
- `tone_profile`
- `style_constraints`
- `audience_profile`

#### `SectionNode`

- `id`
- `parent_id`
- `title`
- `level`
- `start_offset`
- `end_offset`
- `summary`
- `risk_level`

#### `TermRule`

- `term`
- `preferred_form`
- `aliases`
- `do_not_rewrite`
- `notes`

#### `ProtectionRule`

- `rule_type`
  - citation
  - number
  - formula
  - reference
  - named_entity
- `selector`
- `reason`

### 5.2 规划模型

#### `RewritePlan`

- `plan_id`
- `document_id`
- `global_goal`
- `strategy_summary`
- `target_strength`
- `execution_order`

#### `ChunkPlan`

- `chunk_id`
- `section_id`
- `priority`
- `rewrite_strength`
- `context_before`
- `context_after`
- `must_keep`
- `must_avoid`

### 5.3 执行模型

#### `ChunkContext`

- `document_summary`
- `section_summary`
- `previous_chunk_summary`
- `next_chunk_summary`
- `term_rules`
- `protection_rules`
- `local_goal`

#### `ChunkRewriteResult`

- `chunk_id`
- `source_text`
- `candidate_text`
- `diff_spans`
- `rationale`
- `risk_flags`
- `quality_score`

### 5.4 全局复核模型

#### `GlobalIssue`

- `issue_id`
- `issue_type`
  - terminology_drift
  - citation_damage
  - tone_drift
  - logic_break
  - duplicate_expression
- `severity`
- `related_chunk_ids`
- `description`
- `suggested_action`


## 6. 执行流程重构

建议把主流程从当前的：

`导入 -> 切段 -> 改写 -> diff -> 导出`

升级为：

`导入 -> 文档分析 -> 生成规划 -> 带上下文改写 -> 全局复核 -> 人工确认 -> 导出`

### Step 1. 导入与结构识别

- 导入原文
- 标准化文本
- 粗分 section
- 粗分 chunk

### Step 2. 全文分析

- 生成全文摘要
- 提取章节摘要
- 提取术语与禁改项
- 判断高风险区域

### Step 3. 生成改写计划

- 给出整篇改写强度建议
- 给出 section 级策略
- 给出 chunk 级执行顺序

### Step 4. 局部执行

- 按计划挑选 chunk
- 组装 chunk context
- 调模型改写
- 记录解释、diff、风险标记

### Step 5. 全局统一检查

- 检查术语漂移
- 检查风格漂移
- 检查前后逻辑衔接
- 检查引用和数据一致性

### Step 6. 人工审阅

- 用户按风险优先级查看
- 批量接受低风险段
- 单独处理高风险段

### Step 7. 最终导出

- 导出最终稿
- 可选导出审阅记录
- 可选导出变更报告


## 7. Prompt 策略重构

当前 prompt 只告诉模型“改写这一段”。

这不够。

每次改写都应该使用三层 prompt：

### 7.1 System Prompt

定义通用角色：

- 保持原意
- 不改事实
- 不动引用和数字
- 不破坏论文结构
- 优先自然化而非堆砌同义替换

### 7.2 Document Prompt

定义全文约束：

- 全文主题
- 风格目标
- 术语统一规则
- 不可修改项

### 7.3 Chunk Prompt

定义局部任务：

- 当前段位置
- 当前章节目标
- 前后文摘要
- 当前段的具体改写目标

这样局部执行才会服从全文。


## 8. UI 需要怎么重构

当前 UI 已经从平铺改成渐进式，这是正确方向。

但如果目标是全局 agent，UI 还要继续升级。

### 8.1 现在这版 UI 适合什么

适合：

- 工具型改写器
- 逐段人工审阅
- 单次任务推进

不适合：

- 文档级策略理解
- 全局风险控制
- 论文结构导航

### 8.2 目标 UI 的主框架

建议改成 3 层视角。

#### A. 全局总览层

显示：

- 文档类型
- 全文摘要
- 章节树
- 术语表
- 风险总数
- 当前执行阶段

#### B. 执行控制层

显示：

- 改写计划
- section 状态
- chunk 进度
- 自动 / 手动模式
- 暂停 / 恢复 / 重新规划

#### C. 审阅决策层

显示：

- 高风险问题列表
- 片段 diff
- 原文 / 候选文 / 解释
- 批量操作
- 最终稿预览

### 8.3 需要新增的界面模块

#### `Document Overview`

- 全文摘要
- 章节地图
- 改写目标
- 术语规则

#### `Plan Board`

- 每章策略
- 每段优先级
- 高风险区域

#### `Risk Review Panel`

- 风险列表
- 过滤器
- 按严重度排序

#### `Chunk Inspector`

- 原文
- 候选文
- diff
- 改写理由
- 风险标志

#### `Global Consistency Board`

- 术语不一致
- 风格不一致
- 引用问题
- 逻辑问题


## 9. 推荐的工程迭代顺序

不要一次性全改完。

建议分 4 个阶段。

### Phase 1. 从分段工具升级为文档工作台

目标：

- 新增全文摘要
- 新增章节树
- 新增术语表
- UI 新增文档总览页

本阶段不改执行逻辑，只补全局视角。

### Phase 2. 引入规划层

目标：

- 新增 `RewritePlan`
- 新增 section / chunk 级策略
- 改写前先做规划
- UI 新增 `Plan Board`

### Phase 3. 引入带上下文的执行层

目标：

- 每段改写不再只传本段
- 接入全文摘要、章节摘要、术语表和禁改规则
- 输出改写理由和风险标志

### Phase 4. 引入全局复核层

目标：

- 新增全局问题扫描
- 新增术语统一检查
- 新增风格漂移检查
- 新增批量审阅策略


## 10. 对论文场景的特殊要求

如果产品真的要支持论文去 AI 化，必须再补 4 项专用能力。

### 10.1 结构保护

- 摘要、引言、方法、结果、结论要分开处理
- 参考文献区域默认禁改

### 10.2 引用保护

- 不允许改动引用编号
- 不允许破坏括号、年份、作者名格式

### 10.3 术语保护

- 专有术语不能在不同段落被改成不同说法

### 10.4 学术语气约束

- 不能为了“去 AI 味”把学术表达改得口语化
- 论文去 AI 的目标是“自然、可信、稳定”，不是“随意”


## 11. 验收标准

当以下条件满足时，才能说产品已经接近“全局视野去 AI agent”。

- 用户能在导入后看到全文摘要和章节结构
- 用户能看到系统生成的改写计划
- 系统每次改写都能说明依据了哪些全局约束
- 用户能看到术语和引用保护规则
- 用户能在全局问题面板中看到一致性风险
- 最终导出前系统会做文档级统一检查
- 用户能按风险而不是按顺序来审阅


## 12. 下一步建议

如果继续开发，最优先的顺序是：

1. 先补 `DocumentBrief / SectionTree / TermRule / RewritePlan` 数据模型
2. 再补文档总览页和规划页
3. 然后重写 `rewrite_chunk` 的上下文组装逻辑
4. 最后再做全局一致性检查和风险面板

不要反过来。

如果先继续微调当前 diff 审阅页，只会把局部工作台越做越精细，但离“全局视野 agent”还是差一层。
