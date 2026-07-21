# Aerina 项目方案 v1.1

## 一、项目定义

**Aerina** 是一款本地优先、跨平台的多模型 AI 工作台，覆盖：

* 单模型对话
* 图像生成与多模态内容
* 多模型并列比较（SBS）
* 多样化匿名竞技场
* 个人排行榜
* 对话树与 Fork
* 本地统计与使用分析
* 后续可扩展的本地 Agent 与 Agent 竞技场

产品定位：

> Local-first AI workbench, personal model arena, and future agent arena.
> 本地优先的 AI 工作台、个人模型竞技场，以及未来的 Agent 竞技场。

品牌口号：

> **Your models. Your arena.**
> **Different Minds. Better Answers.**

Aerina 不依赖官方云端才能运行。关闭账号、同步和 API Key 上传后，用户仍可完整使用：

* 单模型对话
* 图像生成
* 手动多模型并列对比
* 随机匿名竞技场（文本 / 生图 / 多模态等）
* 本地个人排行榜
* 对话分支与 Fork
* 本地统计面板
* 本地模型和第三方 API
* 数据导入、导出与备份

---

# 二、产品核心原则

## 1. 本地数据库是主数据源

聊天记录、生图记录、竞技场记录、排行榜事件、统计事件、模型配置、分支关系首先写入本地数据库。

云端只提供可选同步，不作为应用运行前提。

## 2. 用户无需登录

首次启动直接创建本地 Profile 和 Workspace。

用户登录云账户，只代表开启跨设备同步，不改变本地数据所有权。

## 3. 同步默认关闭

设置中存在两个独立开关：

| 设置 | 默认状态 |
| --- | --- |
| 云同步 | 关闭 |
| 加密同步 API Key | 关闭 |

普通同步可以开启而不上传 API Key。

## 4. 随机多模型请求才属于竞技场

* 用户手动选择多个模型：属于 **SBS 公开对比**
* 系统随机抽取多个模型：属于 **匿名竞技场**
* 只有匿名竞技场结果进入个人排行榜

## 5. 所有回答都能成为独立对话分支

用户选择最佳回答后，可以：

* 使用最佳模型继续直接对话
* 以最佳回答为上下文继续下一轮竞技场
* 从任意候选回答创建新的 Fork 对话

## 6. 生成能力按 Capability 扩展，而不是按页面分裂

文本对话、图像生成、多模态理解、后续 Agent 运行，都挂在同一套：

* Conversation / Branch / Round / Candidate
* Generation Engine
* Provider 统一接口

不要为生图、Agent、竞技场各自维护一套消息系统。

## 7. Agent 是可插拔运行时，不是 MCP 工具壳

后续 Agent 指的是类似：

* Codex
* Claude Code
* OpenCode
* Pi
* Hermes Agent

这类 **本地任务型 Agent Runtime**，而不是“给聊天模型挂几个 MCP tool”的传统 tool-agent。

MCP 可以是 Agent Runtime 的可选工具源之一，但不是 Agent 的定义本身。

## 8. 统计事件是一等公民

所有生成、投票、费用、延迟、失败、Agent 步骤都产生可查询的统计事件。

统计面板读取事件与派生缓存，不依赖前端临时计算。

---

# 三、核心能力矩阵

| 能力 | 第一版 | 后续版本 |
| --- | --- | --- |
| 直接文本对话 | 是 | 增强 |
| 图像生成 | 是 | 增强 |
| 图文多模态输入 | 是 | 增强 |
| SBS 多模型公开对比 | 是 | 增强 |
| 匿名文本竞技场 | 是 | 增强 |
| 匿名生图竞技场 | 是 | 增强 |
| 多模态竞技场 | 是 | 增强 |
| 本地个人排行榜 | 是 | 增强 |
| 使用统计与费用统计 | 是 | 增强 |
| 本地 Agent Runtime | 否 | 是 |
| Agent 竞技场 | 否 | 是 |
| MCP 作为工具源 | 否 | 是 |
| 云同步 | 后期可选 | 是 |

---

# 四、四种产品表面，一套底层结构

Aerina 对外有四种主要产品表面：

```text
1. Chat Surface
   单模型持续对话 / 生图

2. Compare Surface
   SBS 手动多模型公开对比

3. Arena Surface
   随机匿名多模型竞技场

4. Stats Surface
   使用统计、费用、排行榜、趋势
```

后续第五种：

```text
5. Agent Surface
   本地 Agent 任务、轨迹、审批、Agent 竞技场
```

底层始终共用：

```text
Conversation Tree
Round / Candidate
Generation Engine
Provider Registry
Ranking Engine
Analytics Engine
```

---

# 五、三种核心对话模式

## 模式一：直接对话 Chat

用户选择一个模型，与其持续对话。

```text
用户消息
   ↓
指定模型
   ↓
助手回答 / 生成图像
   ↓
继续使用当前分支上下文
```

支持：

* 流式文本输出
* 图像生成结果回写
* 停止生成
* 重新生成
* 编辑消息并创建分支
* 中途切换模型
* 从历史节点创建 Fork
* 模型设置跟随当前对话
* 保存每次生成时的模型与参数快照

切换模型只影响之后的消息，不修改历史回答。

### 直接对话中的生图

用户可：

* 在文本模型中发送图片做 Vision 理解
* 切换到图像模型生成图片
* 在同一对话树中混排文本节点和图像节点

生图结果作为结构化 Content Block 进入消息树，而不是外挂附件列表。

---

## 模式二：SBS 多模型公开对比

SBS 即 Side-by-Side。

用户手动选择两个或更多模型，相同问题同时发送给所有模型，模型名称始终公开。

```text
                     ┌→ Claude → 回答 A
同一条用户消息 ──────┼→ Gemini → 回答 B
                     └→ GPT    → 回答 C
```

用户可以配置：

* 参与模型数量
* 具体参与模型
* 模型显示顺序
* 每个模型使用的 Model Preset
* 系统提示词
* 是否记录费用、延迟和 Token
* 最大并发请求数
* 输出模态要求（text / image / mixed）

每个模型只看到：

* 当前分支的公共历史
* 当前用户消息
* 自己生成的当前回答

模型不会看到同轮其他模型的回答。

### 回答完成后的操作

用户可以：

1. **选择一个回答继续对话**
2. **继续进行多模型对比**
3. **从任意回答 Fork**

SBS 对比不更新竞技场排行榜，避免用户手动选择阵容造成排名偏差。

SBS 仍写入统计事件，供费用、延迟、成功率分析使用。

---

## 模式三：随机匿名竞技场 Arena

用户指定候选模型池和每轮参与数量，由 Aerina 随机抽取模型。

回答完成前以及投票前，用户看不到模型名称、供应商或图标。

```text
候选池：8 个模型
每轮数量：3 个

随机抽取
   ↓
回答 A    回答 B    回答 C
未知模型  未知模型  未知模型
   ↓
用户选择最佳回答
   ↓
揭晓模型身份
   ↓
更新本地个人排行榜
```

### 竞技场设置

设置保存在当前对话中：

* 竞技场类型
* 候选模型池
* 每轮参与模型数量
* 竞技场分类
* 系统提示词
* 模型能力要求
* 是否允许相同供应商
* 显示列数
* 费用或上下文长度限制
* 随机顺序
* 评分维度

实际抽中的模型每轮重新随机，避免第一轮揭晓后失去匿名性。

### 匿名规则

生成开始时，前端只接收：

```text
Candidate A
Candidate B
Candidate C
```

真实 Model ID 和 Provider 信息保留在 Rust Core 中。

前端只有在用户完成选择后才能获得模型映射，防止界面、日志或网络事件意外泄露答案。

### 投票方式

第一版支持：

* 选择一个最佳回答
* 全部不理想
* 本轮跳过

后续可扩展：

* 多维度评分
* 部分排序
* 并列最优

完成选择后揭晓全部模型身份。

其中：

* 选择最佳回答：更新排名
* 全部不理想：记录质量事件，但不计算模型间胜负
* 跳过：不更新排名

---

# 六、多样化竞技场

竞技场不是只有“文本盲测”一种。

第一版把 Arena 设计成 **Arena Kind + Capability Filter + Scoring Profile**。

## 1. Arena Kind

| Kind | 说明 | 第一版 |
| --- | --- | --- |
| `text` | 纯文本回答比较 | 是 |
| `image_gen` | 同一提示词生图比较 | 是 |
| `vision` | 带图片输入的理解/分析比较 | 是 |
| `mixed` | 文本 + 图片混合输出 | 预留 |
| `code` | 编程任务专项 | 预留 UI，数据可落 |
| `agent_task` | 本地 Agent 任务竞技 | 后续版本 |

## 2. 文本竞技场

经典匿名文本比较。

适用：写作、推理、翻译、总结、问答。

## 3. 生图竞技场

同一提示词发给多个图像模型：

```text
Prompt / 参考图
   ↓
Image Model A / B / C
   ↓
匿名图片候选
   ↓
用户投票
   ↓
揭晓模型并更新图像榜
```

第一版支持：

* 纯文生图
* 可选参考图
* 尺寸 / 比例预设
* 负面提示词（如果 Provider 支持）
* 匿名展示，隐藏水印可识别供应商信息（前端层尽量避免暴露）

## 4. Vision / 多模态竞技场

用户提供图片 + 问题，随机抽取支持 Vision 的模型比较。

## 5. 后续 Agent 竞技场

后续版本支持：

```text
同一任务说明
   ↓
Agent Runtime A / B / C
   ↓
各自独立工作区 / 轨迹
   ↓
用户或 Judge 选择最佳结果
```

Agent 竞技场比较的是：

* 最终交付物
* 轨迹质量
* 步数与耗时
* 工具使用效率
* 是否完成目标

不是比较单次 chat completion。

## 6. Arena Profile

每个竞技场对话保存一个 `ArenaProfile`：

```text
ArenaProfile
├─ kind
├─ category
├─ candidate_pool
├─ slot_count
├─ capability_requirements
├─ allow_same_provider
├─ scoring_profile
├─ reveal_policy
└─ continuation_policy
```

这样后续加新竞技场类型时，只扩展 Profile 和评分器，不重写对话树。

---

# 七、选择最佳回答后的上下文规则

这是 Aerina 最关键的产品逻辑。

## 1. 使用最佳模型继续直接对话

假设竞技场产生：

```text
回答 A → Model X
回答 B → Model Y
回答 C → Model Z
```

用户选择回答 B 后：

```text
原公共上下文
    +
回答 B
    ↓
切换到直接对话
    ↓
默认继续使用 Model Y
```

当前对话的 Active Branch 指向回答 B。

之后发送的新消息，会使用：

* 选择回答 B 之前的公共上下文
* 回答 B 的完整可见内容
* 后续用户消息
* Model Y 对应的 Model Preset

不会把回答 A 和回答 C 加入上下文。

对生图竞技场：

* 选中图片进入主线
* 后续可基于该图继续对话、再编辑提示词、或切到 Vision 模型理解

## 2. 从任意候选回答 Fork

模型身份揭晓后，每个回答都显示：

```text
继续与此模型对话
Fork 为新对话
```

新 Fork 不复制整份历史数据，而是引用共同祖先节点，并将该候选设置为新的分支头。

## 3. 继续下一轮竞技场

用户也可以选择：

```text
以最佳回答继续竞技场
```

此时最佳回答被提交到公共主线，下一轮随机抽取的新模型都会收到：

```text
之前的公共对话
+
上一轮最佳回答
+
本轮用户新消息
```

未获胜的回答仍保留为可回溯的候选分支，但不会进入下一轮公共上下文。

## 4. 全部不理想

用户选择“全部不理想”时：

* 不提交任何候选回答
* 当前分支仍停留在本轮用户消息之前
* 用户可以修改问题后重新生成
* 可以结束本轮并查看模型身份
* 不更新 Elo

---

# 八、统一对话树模型

Aerina 不使用简单的线性消息列表，而是使用对话树。

```text
Conversation
└─ Branch
   ├─ User Message
   ├─ Round
   │  ├─ Candidate A
   │  ├─ Candidate B
   │  └─ Candidate C
   ├─ Selected Candidate
   └─ Next User Message
```

核心概念：

| 概念 | 含义 |
| --- | --- |
| Conversation | 一个对话项目 |
| Branch | 一条独立上下文路径 |
| Round | 一次用户提问及其全部候选生成 |
| Candidate | 某个模型/Agent 在该轮的回答或轨迹摘要 |
| Commit | 将一个候选回答加入当前主线 |
| Fork | 从候选回答创建新的上下文分支 |
| Active Branch | 当前正在继续的对话路径 |
| Content Block | 消息中的结构化内容单元 |

三种模式共用同一套底层结构：

| 模式 | 每轮 Candidate 数量 | 是否隐藏模型 | 是否计入排名 |
| --- | ---: | --- | --- |
| 直接对话 | 1 | 否 | 否 |
| SBS 手动对比 | 2 个以上 | 否 | 否 |
| 随机竞技场 | 2 个以上 | 是 | 是 |

## Content Block

消息内容从第一版就采用结构化 Block：

```text
ContentBlock
├─ text
├─ image
├─ file_ref
├─ code
├─ tool_call        # 后续
├─ tool_result      # 后续
├─ agent_step       # 后续
└─ usage_meta
```

这样同一条 Candidate 可以包含：

* 纯文本
* 一张或多张生成图
* 后续 Agent 步骤摘要

---

# 九、图像生成设计

## 1. 生图不是旁路功能

生图走 Generation Engine，与文本生成共享：

* 取消
* 重试
* 费用记录
* Round 快照
* 分支 / Fork
* SBS / Arena

## 2. Image Generation Request

```text
ImageGenerationRequest
├─ prompt
├─ negative_prompt?
├─ reference_images[]
├─ size
├─ aspect_ratio
├─ seed?
├─ steps?
├─ guidance?
└─ provider_specific_params
```

## 3. Image Generation Result

```text
ImageGenerationResult
├─ images[]
│  ├─ local_path / blob_ref
│  ├─ width
│  ├─ height
│  ├─ mime
│  └─ revised_prompt?
├─ usage
└─ latency
```

图片本体存本地对象存储目录，SQLite 只存元数据和引用。

## 4. 与对话混排

允许：

```text
用户：帮我设计一张海报
模型：先给 3 个文案方向
用户：用方向 2 生图
模型：返回图片
用户：把背景改成深蓝
```

每一轮仍然是 Round + Candidate。

## 5. Provider 能力标签

```text
text
vision
image_generation
streaming
reasoning
tool_calling
agent_runtime
```

竞技场和模型池按能力过滤，不会把纯文本模型抽进生图局。

---

# 十、设置跟随对话

Aerina 使用三级配置。

## 全局默认设置

仅用于创建新对话，例如：

* 默认模式
* 默认模型
* 默认竞技场类型
* 默认竞技场模型数量
* 默认候选池
* 默认系统提示词
* 默认温度
* 默认生图尺寸
* 默认上下文策略

修改全局默认设置，不影响已有对话。

## 对话设置

跟随当前 Conversation：

* 当前模式
* 手动模型列表
* 随机候选模型池
* 每轮参与数量
* 系统提示词
* 模型参数
* 竞技场类别 / Kind
* 生图参数
* 上下文长度策略
* 并发数量
* 显示布局

重新打开对话时完整恢复。

## Round 快照

每次发送消息时，将当时设置保存成不可变快照：

* 实际参与模型
* Model Preset
* 参数
* 系统提示词
* 生图参数
* 随机顺序
* 模型能力信息
* 供应商
* Token 与费用
* 请求时间和响应时间

之后用户修改对话设置，不会改变历史 Round。

---

# 十一、个人排行榜设计

排行榜完全属于当前用户和 Workspace。

数据来源只有：

```text
匿名竞技场 Round
+
用户投票
+
模型身份揭晓结果
```

SBS 手动比较、普通点赞和直接对话均不计入排行榜。

## 排名对象

数据结构同时保留：

* Canonical Model ID
* Provider ID
* Model Preset ID
* Arena Kind

第一版默认展示：

* 综合文本榜
* 生图榜
* 分类榜

后续可增加：

* 模型预设排行榜
* 供应商排行榜
* 性价比排行榜
* Agent 任务榜

## 竞技场分类

用户可为对话指定分类：

* 综合
* 编程
* 中文写作
* 数学
* 翻译
* 推理
* 创意
* 设计 / 生图
* 自定义分类

同一场比赛可以计入一个主分类和综合榜。

## 第一版算法

使用多候选 Elo：

* 最佳候选视为分别战胜其他候选
* 多模型比赛会对每组胜负进行归一化，避免模型数量越多单局分数变化越大
* “全部不理想”和“跳过”不产生胜负
* 排行榜同时显示场次数和置信程度

排行榜本身是派生缓存，可以随时根据竞技场事件重新计算。

不同 Arena Kind 默认分榜，不把文本胜负和生图胜负混成一个无意义分数。

---

# 十二、统计系统设计

统计不是排行榜的附属 UI，而是独立 Analytics 子系统。

## 1. 统计原则

* 所有关键行为落 `AnalyticsEvent`
* 排行榜、仪表盘、趋势图都从事件或派生表读取
* 允许重建
* 前端不做权威统计

## 2. 事件类型

```text
generation_started
generation_completed
generation_failed
generation_cancelled
image_saved
arena_vote_cast
arena_revealed
candidate_committed
branch_forked
model_switched
provider_error
cost_recorded
agent_run_started        # 后续
agent_step_finished      # 后续
agent_run_completed      # 后续
```

## 3. 第一版统计面板

### 总览

* 今日 / 本周 / 本月请求数
* 总 Token
* 总费用
* 平均 TTFT
* 平均总耗时
* 成功率
* 生图次数
* 竞技场局数
* 投票完成率

### 模型维度

* 使用次数
* 胜率（仅竞技场）
* Elo
* 平均费用
* 平均延迟
* 失败率
* 按 Arena Kind 拆分表现

### Provider 维度

* 请求量
* 错误率
* 平均延迟
* 费用占比

### 对话 / 模式维度

* Chat / SBS / Arena 使用占比
* 文本 vs 生图占比
* 各分类竞技场活跃度

### 时间趋势

* 每日费用曲线
* 每日请求量
* 模型使用变化
* 竞技场活跃度

## 4. 统计与隐私

* 默认全部本地
* 不同步原始 prompt 到云，除非用户开启对话同步
* 统计聚合结果默认也不上传
* 导出统计报表是用户主动行为

## 5. 统计实现边界

```text
analytics-engine
├─ event ingest
├─ aggregate rebuild
├─ query API
└─ export
```

不要在 React 组件里散落统计逻辑。

---

# 十三、本地 Agent 与 Agent 竞技场扩展架构

第一版不实现完整 Agent，但架构必须让后续接入时不推翻对话树。

## 1. Agent 是什么

Aerina 中的 Agent 指：

> 可在本地执行多步任务、可操作工具/文件/终端、可产生轨迹、可被审批、可被比较的运行时实体。

它更接近：

* Codex
* Claude Code
* OpenCode
* Pi
* Hermes Agent

而不是：

* 单纯 Function Calling 包装
* 单纯 MCP client 壳

## 2. 分层

```text
UI Surface
   │
Agent Orchestrator
   │
Agent Runtime Adapter
   ├─ builtin runtime
   ├─ external local runtime
   └─ future remote runtime (明确非默认)
   │
Tool / Workspace / Approval / Trajectory
```

## 3. 关键实体

```text
AgentDefinition
├─ id
├─ runtime_kind
├─ display_name
├─ capabilities
├─ default_tools
└─ config

AgentRun
├─ id
├─ conversation_id
├─ branch_id
├─ agent_definition_id
├─ task_prompt
├─ status
├─ workspace_ref
├─ started_at
└─ finished_at

AgentStep
├─ id
├─ agent_run_id
├─ step_index
├─ kind            # think / tool / edit / command / message
├─ input
├─ output
├─ status
└─ timestamps

AgentTrajectory
└─ ordered AgentStep[]
```

## 4. 与对话树的关系

Agent 不另起一套聊天库。

推荐映射：

```text
Conversation
└─ Branch
   └─ Round
      └─ Candidate
         ├─ summary message
         ├─ result artifacts
         └─ agent_run_id
```

也就是：

* 用户看到的是 Candidate 摘要和最终产物
* 详细轨迹进入 `AgentRun` / `AgentStep`
* 需要时再展开轨迹面板

这样 Chat / SBS / Arena 仍然统一。

## 5. Agent Runtime Adapter

统一接口：

```text
AgentRuntime
├─ describe
├─ validate_config
├─ start_run
├─ continue_run
├─ cancel_run
├─ subscribe_events
└─ export_trajectory
```

不同本地 Agent 通过 Adapter 接入：

```text
adapters/
├─ aerina_builtin/
├─ codex_compatible/
├─ claude_code_compatible/
├─ opencode_compatible/
├─ pi_compatible/
└─ hermes_compatible/
```

Adapter 负责：

* 启动本地进程或调用本地协议
* 规范化事件
* 映射审批点
* 收集产物和轨迹

## 6. Approval Engine

本地 Agent 需要明确审批边界：

```text
ApprovalPolicy
├─ auto
├─ confirm_dangerous
├─ confirm_all
└─ deny_network / deny_fs 等能力开关
```

审批事件进入轨迹，也可进入统计。

## 7. Agent 竞技场

后续：

```text
同一 Task Spec
   ↓
随机或手动选择多个 Agent Runtime / 配置
   ↓
各自隔离 workspace
   ↓
并行或顺序执行
   ↓
匿名展示结果与关键指标
   ↓
用户投票
   ↓
更新 Agent 排行榜
```

比较维度示例：

* 是否完成任务
* 结果质量
* 步数
* 耗时
* 费用
* 人工介入次数
* 破坏性操作次数

## 8. MCP 的位置

MCP 是工具协议，不是 Agent Runtime。

正确关系：

```text
Agent Runtime
   └─ may use MCP servers as tool sources
```

因此：

* 先有 Agent Runtime 抽象
* 再让 runtime 可选接入 MCP
* 不要把 Aerina Agent 做成“MCP 中心”

## 9. 第一版为 Agent 预留但不实现

必须预留：

* Content Block 可扩展
* Candidate 可挂 `run_ref`
* Provider/Runtime 注册表可扩展
* Analytics 事件可扩展
* Arena Kind 可扩展到 `agent_task`

第一版明确不做：

* 内置完整 coding agent
* Agent 竞技场 UI
* 默认接入外部 agent 二进制

---

# 十四、主要页面

## 1. 首页

显示：

* 新建对话
* 最近对话
* 最近竞技场
* 最近生图
* 本地排行榜摘要
* 今日费用 / 请求摘要
* 模型服务状态
* 同步状态

## 2. 新建对话

提供入口：

```text
直接对话
选择一个模型持续交流

图像生成
选择图像模型开始创作

多模型对比
手动选择多个模型，并列查看回答

匿名竞技场
从模型池随机抽取，投票后揭晓身份
  ├─ 文本竞技场
  ├─ 生图竞技场
  └─ Vision 竞技场
```

后续增加：

```text
Agent 任务
Agent 竞技场
```

## 3. 直接对话页

单列聊天布局：

* 顶部模型选择器
* 对话设置
* 分支导航
* 消息区域
* 输入框
* 当前上下文和费用提示
* 附件 / 图片输入

## 4. 生图页 / 生图模式

可以是对话页的模式态，不必强行拆成完全独立产品。

需要：

* 提示词输入
* 参考图
* 尺寸比例
* 历史图片瀑布流
* 重新生成
* 送入 Vision 对话
* 送入生图竞技场

## 5. SBS 对比页

桌面端使用动态多列布局。

* 2 个模型：双列
* 3～4 个模型：网格
* 更多模型：横向滚动或标签页

移动端默认一次展示一个候选，通过滑动或顶部标签切换。

图像候选以图片卡片为主，文本候选以消息流为主。

## 6. 竞技场页

结构与 SBS 共用，但隐藏：

* 模型名称
* Provider
* 模型图标
* 价格信息
* 可识别的主题色

回答完成后，底部固定显示投票栏。

投票后显示：

* 模型身份
* 最佳回答
* Elo 变化
* 延迟和费用
* 继续与最佳模型对话
* 继续竞技场
* 从任意候选 Fork

## 7. 排行榜页

展示：

* 综合文本排名
* 生图排名
* 分类排名
* 胜率
* 场次
* Elo
* 平均首字延迟
* 平均总耗时
* 平均费用
* 最近表现趋势

## 8. 统计页

展示：

* 使用总览
* 费用分析
* 模型对比
* Provider 健康度
* 模式占比
* 导出报表

## 9. 模型与 Provider 页

管理：

* Provider
* Base URL
* API Key
* 模型列表
* Model Preset
* 能力标签
* 是否加入随机池
* 费用配置
* 图像默认参数

后续增加：

* Agent Runtime 注册
* Agent 工具权限

## 10. 设置页

分成：

* 常规
* 对话
* 生图
* 竞技场
* 统计
* 数据与备份
* 隐私
* 云同步
* API Key 同步
* 外观
* 后续：Agent

---

# 十五、技术栈

## 客户端

```text
Tauri 2
React
TypeScript
Vite
Tailwind CSS
shadcn/ui
Zustand
TanStack Query
React Virtuoso
```

Tauri 2 官方支持从同一套项目构建 Windows、macOS、Linux、Android 和 iOS 应用，并允许使用 React 等任意前端框架，适合作为 Aerina 的统一客户端基础。

## 本地核心

```text
Rust
Tokio
Reqwest
Serde
SQLx
SQLite
```

SQLx 提供 SQLite 驱动并支持 Tokio 异步运行时，适合让本地数据库、模型请求和流式生成统一留在 Rust Core。

SQLite 开启 WAL 模式。

## 本地敏感信息

API Key 不直接存入普通 SQLite 字段。

统一通过 `SecretStore` 接口保存到：

* Windows Credential Manager
* macOS/iOS Keychain
* Android Keystore
* Linux Secret Service
* Stronghold 后备存储

SQLite 只保存 Secret Reference。

## 本地对象存储

```text
attachments/
generated-images/
agent-workspaces/   # 后续
exports/
```

大文件不进 SQLite。

## 云端同步服务

```text
Rust
Axum
Tokio
SQLx
PostgreSQL
S3-compatible Object Storage
```

---

# 十六、客户端基础架构

```text
React UI
   │
   │ Tauri Commands / Channels
   ▼
Rust Application Core
   ├─ Conversation Engine
   ├─ Generation Engine
   ├─ Arena Engine
   ├─ Branch Engine
   ├─ Ranking Engine
   ├─ Analytics Engine
   ├─ Provider Registry
   ├─ Context Builder
   ├─ Secret Store
   ├─ Local Storage
   ├─ Media Store
   ├─ Optional Sync Engine
   └─ Optional Agent Orchestrator
          │
          ├─ SQLite
          ├─ Model APIs
          ├─ Image APIs
          ├─ Local Models
          ├─ Local Agent Runtimes
          └─ Cloud Sync
```

前端禁止直接：

* 访问 SQLite
* 持有 API Key
* 调用模型供应商
* 计算 Elo
* 决定竞技场真实模型身份
* 直接启动本地 Agent 进程

这些逻辑全部由 Rust Core 管理。

---

# 十七、核心模块边界

```text
domain
核心实体和业务规则

conversation-engine
对话、Round、Branch 和 Fork

generation-engine
并发生成、流式事件、取消与重试、文本/生图统一调度

provider-registry
模型供应商、图像供应商、协议适配

context-builder
根据当前 Branch 构造模型上下文

arena-engine
随机抽取、匿名映射、投票和揭晓、多 Arena Kind

ranking-engine
个人 Elo 和排行榜重算

analytics-engine
事件采集、聚合、查询、导出

storage
SQLite、迁移和 Repository

media
图片与附件本地存储

secrets
本地密钥安全存储

sync
可选增量同步

import-export
本地备份和数据迁移

agent-orchestrator      # 后续
agent-runtime-adapters  # 后续
approval-engine         # 后续
trajectory-store        # 后续
```

消息内容从第一版就采用结构化 Block，以便增加图片、文件、工具调用和 Agent 步骤。

---

# 十八、核心数据实体

```text
Profile
Workspace
Provider
Model
ModelPreset
AgentDefinition          # 后续

Conversation
ConversationSettings
ArenaProfile
Branch
MessageNode
ContentBlock
Round
CandidateGeneration
ArenaVote
RankingEvent
AnalyticsEvent

Attachment
GeneratedImage
UsageRecord
MediaObject

AgentRun                 # 后续
AgentStep                # 后续
AgentTrajectory          # 后续

SyncOperation
Device
```

关键关系：

```text
Conversation
├─ Branch A
│  ├─ Round 1
│  │  ├─ Candidate A
│  │  ├─ Candidate B
│  │  └─ Candidate C
│  └─ Selected Candidate
│
└─ Branch B
   └─ Forked Candidate
```

每个 `CandidateGeneration` 都保存完整模型/运行时快照，而不是只引用当前设置。

这样模型名称、Base URL、参数、Agent 配置以后改变，旧竞技场仍然可以正确显示和复算。

---

# 十九、Provider 设计

统一接口：

```text
ModelProvider
├─ list_models
├─ validate_config
├─ generate_stream
├─ generate_image
├─ cancel_generation
└─ estimate_capabilities
```

说明：

* 文本供应商实现 `generate_stream`
* 图像供应商实现 `generate_image`
* 多模态供应商可同时实现
* 不支持的能力直接返回明确错误，不做静默降级

第一阶段支持：

* OpenAI Compatible
* OpenAI
* Anthropic
* Gemini
* Ollama
* LM Studio
* 常见图像 API / OpenAI Images 兼容端点
* 自定义 OpenAI Compatible Endpoint

每个 Model Preset 包含能力标签：

* Text
* Vision
* Image Generation
* Streaming
* Reasoning
* Tool Calling
* Context Length
* 后续：Agent Runtime

随机竞技场只会从满足当前请求能力的模型中抽取。

例如：

* 用户发送图片时，不支持 Vision 的模型自动排除
* 生图竞技场只抽 `image_generation`
* Agent 竞技场只抽 `agent_runtime`

---

# 二十、云同步规则

云同步只同步结构化业务数据，不同步整个 SQLite 文件。

```text
本地修改
   ↓
业务表事务
   +
SyncOperation
   ↓
用户开启同步后批量上传
```

同步内容包括：

* Conversation
* Branch
* Message
* Round
* Candidate
* Vote
* Provider 普通配置
* Model Preset
* 附件元数据
* 生成图元数据

排行榜和统计聚合不直接同步。

不同设备同步竞技场记录和投票后，在本地重新计算排行榜与统计。

API Key 使用单独的加密同步模块。

服务器不保存明文 API Key。

---

# 二十一、代码仓库结构

```text
aerina/
├─ apps/
│  ├─ client/
│  └─ client/src-tauri/
│
├─ packages/
│  ├─ ui/
│  ├─ protocol/
│  └─ shared-types/
│
├─ crates/
│  ├─ domain/
│  ├─ application/
│  ├─ conversation/
│  ├─ generation/
│  ├─ providers/
│  ├─ arena/
│  ├─ ranking/
│  ├─ analytics/
│  ├─ media/
│  ├─ storage/
│  ├─ secrets/
│  ├─ sync/
│  ├─ import-export/
│  └─ agent/                 # 后续
│     ├─ orchestrator/
│     ├─ runtime-adapters/
│     ├─ approval/
│     └─ trajectory/
│
├─ services/
│  └─ sync-server/
│
├─ migrations/
├─ assets/
├─ docs/
└─ deploy/
```

第一版保持模块化单体，不拆微服务。

---

# 二十二、开发里程碑

## M0：工程与领域基础

交付：

* Monorepo
* Tauri 基础应用
* SQLite 迁移系统
* Domain 实体
* Repository 接口
* Provider 接口
* 统一流式事件协议
* Conversation / Branch / Round 数据结构
* Content Block
* AnalyticsEvent 基础表

验收重点：底层已经能够表达多候选、Fork、图像块和统计事件，不以单模型线性文本对话为前提。

## M1：直接对话

交付：

* Provider 管理
* API Key 安全存储
* 单模型流式对话
* 对话历史
* 停止与重新生成
* 编辑消息
* 基础分支
* 基础 Usage 记录
* 导入导出

## M2：图像生成与多模态输入

交付：

* 图像 Provider
* 文生图
* 参考图
* 图片本地存储
* 对话中混排图片
* Vision 输入
* 生图费用与耗时统计

## M3：SBS 手动多模型对比

交付：

* 手动选择多个模型
* 并发生成
* 动态多列布局
* 候选状态和单独重试
* 选择一个回答继续
* 从任意回答 Fork
* 模型费用和延迟统计
* 文本与生图 SBS

## M4：多样化随机匿名竞技场

交付：

* 候选池
* 每轮模型数量
* 随机抽取
* 匿名 Candidate
* 随机显示顺序
* 投票后揭晓
* 文本 / 生图 / Vision 竞技场
* 最佳模型上下文继承
* 继续竞技场
* 从任意候选 Fork

## M5：本地个人排行榜与统计面板

交付：

* Elo 计算
* 分 Kind / 分类排行榜
* 排名重算
* 场次和胜率
* 延迟和费用统计
* 本地趋势图
* 竞技场记录回看
* 使用总览仪表盘
* 模型 / Provider 统计
* 统计导出

## M6：数据稳定性

交付：

* 完整备份与恢复
* 数据迁移测试
* 异常退出恢复
* 请求失败恢复
* 大型聊天虚拟化
* 多平台适配
* 自动化测试

## M7：可选云同步

交付：

* 账号和设备
* 增量同步
* Branch 和 Candidate 同步
* 附件 / 图片同步
* 冲突处理
* 加密 API Key 同步
* 远程设备管理

## M8：本地 Agent Runtime 接入

交付：

* AgentDefinition / AgentRun / AgentStep
* Agent Orchestrator
* Approval Engine
* 至少一个本地 Runtime Adapter
* 轨迹面板
* Agent 使用统计

## M9：Agent 竞技场

交付：

* `arena_kind = agent_task`
* 多 Agent 并行任务
* 匿名结果比较
* Agent 排行榜
* 轨迹对比视图

## 更后续

* 更多 Runtime Adapter
* MCP 作为工具源
* 知识库
* 圆桌讨论
* 模型互评
* 自动 Judge
* 自定义竞技场规则
* 插件系统

---

# 二十三、第一版明确不做

为了保证核心稳定，第一版不做：

* 公共全球排行榜
* 官方代理用户的模型请求
* 完整本地 Coding Agent
* Agent 竞技场
* 云端 Agent
* 多人协作
* 复杂 CRDT
* 自动模型裁判
* 模型互相阅读回答
* 插件市场
* 工作流编排
* 知识库/RAG
* 静默兼容旧数据逻辑

数据结构会为这些能力预留，但不进入 MVP。

---

# 二十四、核心验收标准

Aerina 第一正式版本必须满足：

1. 不注册账号也能完整使用对话、生图、SBS、竞技场、排行榜和统计。
2. 云同步和 API Key 同步默认关闭。
3. 关闭 Aerina 云服务后，本地功能不受影响。
4. 手动选择多个模型时，模型名称公开且结果不计入排行榜。
5. 随机选择多个模型时，投票前不能看到任何模型身份信息。
6. 用户投票结束后才揭晓模型名称。
7. 选择最佳回答后，可直接使用该模型和该回答的上下文继续对话。
8. 可以从任意候选回答创建独立 Fork。
9. 继续竞技场时，只有上一轮最佳回答进入公共上下文。
10. 其他候选回答不会污染最佳分支。
11. 每轮保存独立模型和参数快照。
12. 排行榜可由本地竞技场事件完整重算。
13. 统计面板可由本地分析事件完整重算。
14. 历史对话重新打开后，模型池、数量、模式和生图设置正确恢复。
15. 某个候选失败时，不影响其他候选继续生成。
16. 生图结果进入对话树，可 Fork、可继续、可再比较。
17. 文本竞技场与生图竞技场分榜，不混算无意义总分。
18. 移动端可以通过标签或滑动查看全部候选回答。
19. 后续接入本地 Agent 时，不需要改写 Conversation Tree 主模型。

---

# 二十五、不可更改的架构决策

为了避免后续重构，以下规则从项目第一天固定：

1. **Conversation 必须是树，不是线性消息数组。**
2. **Round 必须支持任意数量的 Candidate。**
3. **直接对话、SBS 和 Arena 共用同一个 Generation 系统。**
4. **文本生成与图像生成共用 Generation 调度与 Round 快照。**
5. **竞技场真实模型信息在投票前不能进入前端状态。**
6. **每轮设置使用不可变快照。**
7. **Candidate 必须能够 Commit 或 Fork。**
8. **最佳回答才进入公共主线上下文。**
9. **排行榜由竞技场事件派生，不作为主数据。**
10. **统计由分析事件派生，不作为主数据。**
11. **SQLite 永远是本地主数据源。**
12. **同步和 Agent 永远作为可插拔模块。**
13. **Provider / Runtime 必须通过统一接口接入。**
14. **前端不能持有明文 API Key。**
15. **Agent 以本地 Runtime Adapter 方式扩展，不以 MCP 中心模型定义 Agent。**
16. **Arena 以 Kind + Capability + Scoring Profile 扩展，不按页面复制逻辑。**
17. **不做静默 fallback；能力不支持就显式失败。**

---

# 二十六、最终产品形态

Aerina 的主流程最终应当是：

```text
选择模式
   │
   ├─ 直接对话
   │     ├─ 文本交流
   │     └─ 图像生成 / 多模态
   │
   ├─ 手动多模型对比
   │     ├─ 查看公开模型回答或图片
   │     ├─ 选择一个继续
   │     └─ 从任意回答 Fork
   │
   ├─ 随机匿名竞技场
   │     ├─ 文本 / 生图 / Vision / 后续 Agent
   │     ├─ 随机抽取
   │     ├─ 匿名并列生成
   │     ├─ 用户选择最佳
   │     ├─ 揭晓身份并更新排名
   │     ├─ 与最佳模型继续
   │     ├─ 继续下一轮竞技场
   │     └─ 从任意候选 Fork
   │
   ├─ 统计与排行榜
   │     ├─ 使用量 / 费用 / 延迟
   │     ├─ 模型与 Provider 分析
   │     └─ 分类型个人榜
   │
   └─ 后续：本地 Agent
         ├─ 单 Agent 任务
         ├─ 轨迹与审批
         └─ Agent 竞技场
```

这套设计让 Aerina 不只是“带排行榜的聊天客户端”，而是一个真正围绕：

* 多模型选择
* 多模态生成
* 上下文继承
* 个人判断
* 本地统计
* 未来本地 Agent 对决

构建的 AI 工作台。
