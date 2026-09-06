# 天成科研助手

**本地优先的 AI 科研工作台。** 把文献、数据、算力、绘图、写作和决策放在同一个项目里。

当前版本 **v1.9.0**。桌面应用支持 Windows / macOS / Linux；也提供无界面 CLI。

- 下载安装包：[tctoken.cn/downloads](https://www.tctoken.cn/downloads/)
- GitHub：[imbigfly/SuperScience](https://github.com/imbigfly/SuperScience)
- 本仓库 GitLab：<http://119.96.147.4:8081/huangfei/SuperScience>
- 上游开源项目：[xuzhougeng/wisp-science](https://github.com/xuzhougeng/wisp-science)

数据、会话和项目文件默认留在本机。模型 API Key 进系统钥匙串，不写入 SQLite。

---

## 它解决什么问题

科研过程通常散落在浏览器、终端、网盘和 Word 里：检索一篇、分析一份、出一张图、再另开窗口写稿。天成科研助手把这些步骤收进一个 **Project**：

| 概念 | 含义 |
|------|------|
| Project | 一个课题的工作区（文件、会话、产物） |
| ExecutionContext | 本机 / WSL / SSH 等执行环境 |
| Run | 可跟踪的长任务（日志、状态、收获） |
| Artifact | 带版本与来源的产物 |
| Paper / Decision | 文献与研究决策，可挂到研究图谱 |

助手能读写项目文件、跑 Python / R、调用 Skills 与 MCP 工具。默认走审批门控，需要时再开完整权限。

---

## 我能做什么

首页和设置里的「能力」是同一套卡片，按科研阶段分类。点卡片会开引导对话、打开设置，或进入对应工作面。

### 选题与立项

- **深度学术调研**：带引用的领域简报，而不是搜索链接堆
- **基金 / 开题写作**：立项依据、创新点、任务书提纲
- **文本知识图谱**：从文本抽出实体与关系
- **生物数据库**：PubMed / GEO / UniProt / PDB 等（经内置 MCP）
- **文献矩阵检索 / 引用审计检索 / 文献流水线**
- **合法全文获取**：只走开放获取路径，不绕过付费墙
- **深读论文卡片 / 中英对照阅读器**：精读后留下可打开的对照报告

### 研究设计与规划

- **研究图谱**：数据、文献、产物和决策的关系图
- **文件与收藏库**：项目文件，以及跨会话收藏的代码格 / 图

### 研究实施与数据分析

- **统计报告审核 / 统计建模与检验**
- **Python / R 分析**：持久 REPL，变量可跨 cell 保留
- **远程算力与长任务**：注册 SSH / WSL，提交带日志的 Run
- **结构预测与分子设计 / 单细胞与分析流程**
- **实验日志**：照片、语音或随手记录整理成 Markdown 日志

### 论文写作与发表

- **学术论文写作 / 研究到成稿流水线**
- **国际期刊审稿 / Nature 审稿**
- **Nature 风格起草与润色 / 去除 AI 痕迹**
- **引用补全 / 参考文献核验 / 数据可用性（FAIR）**
- **修回信 / 答复信**
- **论文转专利初稿**
- **Word / Excel / PPT**（OfficeCLI）

侧栏仍可打开论文证据工作区：为某一稿某一版冻结证据并打成可核查胶囊。

### AI 画图

- **投稿级科研作图 / R 语言生信图**
- **论文转组会 PPT / 图片转可编辑 PPT**
- **PPT Master**（首次使用会先安装技能）
- **AI 插图生成**；部分机制图 / 布局入口仍在建设中

### 数据清洗

- **数据清洗与整形**
- **配置本机环境**：Python / R / 依赖探测与准备

### 效率工具

- **出站隐私脱敏**：发往云端模型前假名化邮箱、手机号、身份证和自定义关键词；占位词可编辑，保存时检查格式
- **论文预审 / 选题引导**
- **图片查重**：审核图块是否重复、复用或变换后再用
- **手写数据提取**：识别手写表格，可分别指定识图模型与校准模型
- **知识库**：检索本机 WeKnora 并引用片段

### 协作扩展

- **飞书 / 微信接入**：用 IM 驱动同一桌面会话，工具审批仍在本机
- **真实浏览器**：通过本机 Chrome 扩展操作已登录网页
- **Playwright 无头浏览器**
- **插件扩展**：安装 Skill + MCP 功能包
- **技能扩展**：打开技能目录，浏览、开关或添加 `SKILL.md`

---

## 工作台能力

这些不全部做成首页卡片，但构成日常科研闭环。

**对话与 Agent**

- 流式思考、工具调用、停止与上下文压缩
- `@` 附加文件 / 产物 / 环境，`#` 引用历史会话，`/` 指定 Skill 或 Workflow
- 会话持久化、导出导入、撤销某一轮文件改动
- 可接入 OpenAI 兼容、Anthropic，或通过 ACP 驱动 Codex / Claude Code

**算力与运行**

- 本机始终可用；SSH 主机注册一次即可探测硬件并提交 Run
- 密钥存在系统钥匙串 / secrets 文件，不进项目数据库
- 大文件优先用远程引用，不默认同步到本机

**研究资产**

- Artifact 版本与血缘（代码、日志、输入、环境）
- 探索分支：试一条方向而不改主线
- 全局收藏库、加密手动同步、项目 ZIP 迁移

**扩展**

- Skills：bundled / 项目 / 全局 / 插件，按需 `use_skill`
- MCP：内置约 80 个科学数据库，也可加本地或远程服务
- 专家（Specialists）、多 Agent 委派、可视化 Workflow

上手说明见 [基础配置](docs/basic-configuration.md)。

---

## 开始使用

1. 从 [官网下载页](https://www.tctoken.cn/downloads/) 安装桌面版。
2. 在 **设置 → 模型** 添加 API 与模型。
3. 新建或打开一个项目，从首页「我能做什么」进入课题。

| 平台 | 安装包 |
|------|--------|
| Windows | 已签名 MSI / NSIS |
| macOS | 已签名并公证的 `.dmg`（Apple Silicon + Intel） |
| Linux | `.deb` / AppImage（x86_64 + aarch64） |

示例项目仍可从项目列表进入，用于回看只读演示轨迹。

---

## 从源码运行

需要 Rust 1.88+（含 `wasm32-unknown-unknown`）、[uv](https://docs.astral.sh/uv/)、Trunk、Tauri CLI v2。详见 [开发指南](docs/development.md)。

```bash
rustup target add wasm32-unknown-unknown
cargo install --locked trunk
cargo install tauri-cli --version "^2"

cargo tauri dev      # 热重载
cargo tauri build    # 安装包在 target/release/bundle
```

无界面 CLI：

```bash
export SUPERSCIENCE_API_KEY=<your-key>
export SUPERSCIENCE_PROVIDER=openai
export SUPERSCIENCE_MODEL=deepseek-v4-flash
cargo run -p superscience-cli
cargo run -p superscience-cli -- run "Summarize the files in this project"
```

桌面端认 `SUPERSCIENCE_*`，并继续兼容上游 `WISP_*` 环境变量。远程协议头（如 `__WISP_HANDLE__`）保持原名，以便跟上游合并。

---

## 仓库结构

```text
crates/superscience-core/      Agent 循环、上下文、记忆
crates/superscience-tools/     读改搜、shell 等内置工具
crates/superscience-store/     SQLite 与迁移
crates/superscience-runtime/   持久 Python / R REPL
crates/superscience-skills/    SKILL.md 发现与 use_skill
crates/superscience-llm/       模型调用与出站隐私脱敏
crates/superscience-mcp/       MCP 客户端
crates/superscience-acp/       ACP 外部 Agent
crates/superscience-cli/       无界面命令行
crates/superscience-dto/       UI ⇄ 桌面契约
crates/wisp-runs/              Run 控制面（本地 / SSH）
src-tauri/                     桌面壳、命令、SSH 主机登记
ui/                            Leptos 前端
ui-tests/                      Playwright（模拟 Tauri 桥）
skills/                        预装科研工作流
docs/                          使用与架构文档
report/                        本 fork 上游同步记录
```

---

## 文档

| 主题 | 文档 |
|------|------|
| 上手 | [基础配置](docs/basic-configuration.md) · [模型](docs/model-configuration.md) · [ACP](docs/acp-agents.md) |
| 科研 | [探索分支](docs/exploration-branches.zh-CN.md) · [证据胶囊](docs/publication-evidence.md) · [选题库](docs/case-studies.zh-CN.md) |
| 项目 | [迁移](docs/project-transfer.md) · [同步](docs/project-sync.zh-CN.md) · [全局库](docs/global-library.md) |
| 算力 | [终端](docs/terminal-sessions.md) · [远程文件](docs/remote-file-browser.md) · [传输](docs/server-transfers.md) |
| 扩展 | [Skills](docs/skills.md) · [插件](docs/feature-plugins.md) · [委派](docs/agent-delegation.md) · [IM](docs/channels.md) · [浏览器](docs/real-browser-automation.md) |
| 开发 | [开发指南](docs/development.md) · [更新与发版](docs/app-updates.md) · [功能总览](docs/天成科研助手-完整功能思维导图.md) |

---

## 许可证与来源

除另有说明外，采用 [AGPL-3.0-only](LICENSE)。

本仓库是 [wisp-science](https://github.com/xuzhougeng/wisp-science) 的产品 fork：用户可见品牌为「天成科研助手 / SuperScience」，上游协议字符串与 `WISP_*` 环境变量保持原名，避免下次合入大面积冲突。预装 Skills 的第三方声明见 [skills/THIRD_PARTY_LICENSES.md](skills/THIRD_PARTY_LICENSES.md)。

反馈：应用内 **帮助 → 反馈问题**。会自动附带版本、系统和当前模型。
