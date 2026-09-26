# 上游 bug 修复选择性摘取报告 · v1.12.0–v1.15.0（仅修复，不合版本）

- **日期**：2026-09-26
- **工作分支**：`local/upstream-fixes-202609`（自 `local/brand-tiancheng` `8129e078` 切出，原分支未动）
- **上游范围**：`xuzhougeng/wisp-science` v1.11.0 基线（`5970f2eb`）之后的 140 个提交中，只摘取孤立的 bug 修复；**不引入**任何版本功能（Skill 市场 / Agent Workflow / 上下文 epoch / 便携项目 / 订阅登录 / 原生 SwiftUI·WinUI 界面），不改数据库 schema，不升级依赖（tao 保持 0.35.3）。
- **品牌化**：全部保留。冲突融合规则：上游补丁中的 `wisp_*` crate 引用改写为 `superscience_*`；`WISP_*` / `__WISP_*` 协议标记保持原名；`i18n.rs` / `messages.rs` / `settings_view.rs` 冲突保留本地品牌文案与图标。

## 已摘取（15 个修复）

| PR | 提交 | 修复内容 | 融合说明 |
|---|---|---|---|
| #1235 | `e18c17fb` | 新模型图像输入像素限制（超 2048px 自动生成 JPEG 输入副本） | 只保留 `wisp-tools/image.rs` 核心与 fixture；放弃 `wisp-mcp/result.rs` 部分（依赖 v1.12 #1222 rich-results 架构，本地无） |
| #1237 | `9e0ddd7e` | 新窗口空白会话与运行中审批丢失（水合修复） | 放弃 `limiter.cancelled(epoch)` 子行为（依赖 v1.12 #1225 的 limiter cancel 机制）；保留 `owns_confirmation` 归属检查与 `confirm-resolved` 事件 |
| #1246 | `849fc3dc` | 编辑模型 token 上限时输入焦点丢失 | memo（`model_form_media_kind`）完整落地；表单结构冲突取本地 provider-form-sheet |
| #1249 | `f8d3fc1b` | 同值信号写入导致动画 UI 反复重挂载 | 干净落地 |
| #1255 | `4aa54969` | 连接错误指向已移除的 Model API 代理设置 | 新增 `superscience-llm/src/system_proxy.rs`（OS 系统代理检测，新依赖 `system-configuration`/`windows-registry`）；文案指向本地"设置 → 常规 → 网络"（本地布局与上游一致）；丢弃 OpenCode 相关文案与测试（本地无 #1217） |
| #1258 | `0b381144` | Windows 回复后首次点击误重建 shell | 干净落地 |
| #1260 | `9c323d90` | 会话分支丢失来源文件夹 | store 层修复 + 测试，干净落地 |
| #1268 | `86b9516a` | 会话文件夹移动后分支后代错乱 | store 层修复 + 测试，干净落地 |
| #1269 | `0a39ca6b` | 托管 SSH 明确要求 OpenSSH 8.4+（原为超时） | 新增 `superscience-runs/src/openssh.rs`；原生界面文件（`apps/`、`contracts/`）丢弃 |
| #1270 | `aa8d2b9f` | 建议文件名被误当本地工作区路径 | 干净落地 |
| #1293 | `a952ccc6` | auto-review 改为会话级作用域（开启不再影响下一个会话） | `model_catalog_shared` 取上游（Kimi 命名空间重命名顺带落地）；原生界面文件丢弃 |
| #1305 | `15dc4ad0` | 居中源码编辑器水平平移失灵 | 干净落地 |
| #1313 | `4377e6a2` | figure QA 裁剪图混入输出目录（移入 `.cache/`） | 干净落地 |
| #1318 | `dcd70bd5` | 分支合并摘要使用 profile 输出预算（去掉硬编码上限） | 适配本地 8 元组 `load_session_settings`；`research_archive.rs` 部分丢弃（依赖 v1.14 #1272 研究归档，本地无） |
| #1321 | `f7c4a543` | 已删除文件的聊天路径仍可点击（`WorkspacePathLiveness` 存在性检查） | 调用点保留本地 artifacts 参数；去掉本地 DTO 没有的 `ChatItem::System`/`Checkpoint` 臂；`is_image_model()` 以等价表达式重写；删本地旧版 `code_is_inside_pre`（保留上游增强版） |
| #1332 | `0b246561` | 命令面板方向键超出可视窗口 | 取上游分组渲染结构（导航修复载体）；去掉本地没有的 `star-us` 命令臂（#1239 星标功能未摘） |
| #1359 | `1a71810c` | App 退出序列逐步日志（卡死诊断） | 去掉本地没有的 `mcp_broker` / `mcp_connections` 关停步骤（v1.12 MCP 连接管理重构未跟） |

另有 `59edb23e`（cargo fmt，含 `feedback.rs` 既有漂移清理）与 `af471a3a`（忽略本地 `.zcode/` 会话目录）两个辅助提交。

## 未摘取及原因

| 项 | 原因 |
|---|---|
| #1242 Guide now 循环边界投递 | 上游 queue 重构（`queued_cutins` 优先交接）与本地 `660b3547`（`TurnInFlight` + parking）是同一问题的两套结构性方案，函数签名与 `SessionRuntime` 字段均不兼容；核心注入语义本地已有等价实现（`drain_mid_turn_guidance`） |
| #1285 研究归档草稿输出上限 | 依赖 v1.14 #1272 研究归档功能（新表 + 1000 行模块），本地无 |
| #1337 会话组名浏览器自动填充 | 会话分组（session groups）为 v1.12+ 功能，本地无 |
| #1266 / #1267 / cb0385ab Tao 死锁修复 | 针对上游 Tao 0.37 升级；本地 tao 0.35.3 不适用 |
| #1347 原生会话追问队列 | 纯原生界面（SwiftUI/C#）契约，无 WebView 入口 |

## 可后续补充的关联项

- #1237 的 limiter cancel 子行为：随 #1225（MCP lifecycle）一起摘取时可补。
- #1242：如后续同步 v1.13 或与本地 queue 方案统一时再评估。

## 验证

- `cargo fmt --all -- --check`：通过（本轮引入的漂移已格式化，另含一处既有漂移顺带清理）。
- `cargo check --workspace` / `cd ui && cargo check --target wasm32-unknown-unknown`：通过。
- `cargo test --workspace`：**全部通过**（约 2200 项，0 失败）。
- `cd ui-tests && npm ci && npx playwright test`：**650 通过 / 3 跳过 / 0 失败**（26.7 分钟）。
- 测试代码适配提交：`fe81faae`（`wisp_store` 改名与 `ToolResult.image` 字段适配）、`d7ddf2f1`（`wisp_llm`/`wisp_dto` 测试引用清扫）。
- 手工冒烟建议：多窗口打开运行中会话与待审批项（#1237）、发送大图给新模型（#1235）、SSH 主机低版本 OpenSSH 报错文案（#1269）、命令面板方向键（#1332）、删除文件后聊天路径不再可点击（#1321）。

## 兼容性

- 数据库零改动，无迁移，旧版本数据完全兼容，可随时回退到 `local/brand-tiancheng`。
- 每个修复保留独立提交（含上游 PR 号），可单点 `git rebase --onto` 摘除或整批放弃。
