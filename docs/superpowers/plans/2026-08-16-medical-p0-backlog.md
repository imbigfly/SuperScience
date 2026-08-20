# 医疗科研 P0 产品改动清单

> 依据：[医疗科研视角能力审阅报告](../../../report/2026-08-16-medical-capability-review.md)
>
> 原则：小 PR、一次只加一个可感知入口或一个 durable 抽象；不重写文献/统计内核。首页与设置-能力必须共用 `CapabilitySceneTabs` + `CapabilityTileGrid`。

本清单只拆 **P0 四条工作流**。P1（CONSORT 清单、ChiCTR、NSFC 栏目、PHI 加深）不在本轮开工。PRISMA / 荟萃分析放在临床统计专家之后的独立后续，不并进本四条。

## 交付顺序

1. **首页入口收敛**（体验债，无新依赖，先做）
2. **临床统计专家**（复用持久 R + 现有 specialist 模式）
3. **Zotero + 本地 PDF RAG**（MCP / 插件，不自研引擎）
4. **知网 skill**（复用已有浏览器桥，不假装有官方 API）

每条可独立发版。后三条不要互相阻塞；首页收敛可以先露出「即将接入」卡片，但**禁止再加 `CapabilityAction::None` 占位**。

---

## 工作流 A：首页入口收敛

**用户问题：** 点了没反应的画图卡降低信任；临床试验、PDF 深读、生物医学证据审读已经存在，但首页找不到。

### A1. 去掉三张未接线画图卡

**改动**

- [ui/src/capabilities_home.rs](../../../ui/src/capabilities_home.rs)：从 `capability_catalog()` 删除 `ai-mechanism-figure`、`editable-figure`、`bioinfo-figure-layout`。
- 同步删除 [ui/src/i18n.rs](../../../ui/src/i18n.rs) 中对应 En/Zh title/blurb（若无其它引用）。
- 更新同文件 catalog 测试：`AiDrawing` 组 id 列表不再包含这三张；删除「action == None」断言循环。

**验收**

- 首页 / 设置-能力「AI 画图」只剩已接线卡片：`nature-figure`、`r-bioinfo-figure`、`nature-paper2ppt`、`ppt-master`、`ai-drawing`。
- Playwright：打开能力页，这三张标题不出现。
- 机制图需求继续走「AI 插图」或「投稿级科研作图」，不另开空卡。

### A2. 研究资产：露出已有隐藏 skill

在 `CapabilityGroup::Assets` 增加三张 **GuidedChat** 正卡（不要新写第二套文案体系）：

| id | skill | 建议中文标题 | 作用 |
|---|---|---|---|
| `pdf-explore` | `pdf-explore` | PDF 按页深读 | 已 bundl，首页原先没有独立卡 |
| `audit-biomedical-evidence` | `audit-biomedical-paper-evidence` | 生物医学证据审读 | 逐图审机制/因果，不给临床决策 |
| `literature-review` | `literature-review` | 文献综述与证据 | 与 deep-research / nature-literature-pipeline 去重：本卡强调「先检索再综合」 |

**改动**

- `capability_catalog()` 用现有 `skill_tile(...)`。
- i18n：`caps.skill.pdf_explore.*`、`caps.skill.audit_biomedical_evidence.*`、`caps.skill.literature_review.*`（En + Zh）。
- 引导 prompt 沿用现有「最多五个问题、先要附件/路径」句式。
- 更新 Assets 组长度断言（现 `>= 6`）。

**文献入口收敛（文案，不删 skill）**

首页只保留三条文献主路径，其它 skill 由主路径 `use_skill` 按需加载：

| 用户要做的事 | 首页只留 |
|---|---|
| 查一篇 / 一组文献 | `nature-academic-search` |
| 做带引用的综述 | `literature-review`（本卡） |
| 检索→评分→精读→归档 | `nature-literature-pipeline` |

`deep-research` 留在「论文撰写」（深度调研），不要再在资产组重复一张。`bear-*` 不单独占卡，由 literature-review / 选题对话按需加载。

### A3. 统计分析：露出已有临床试验 MCP

**不要新写 MCP。** bio-tools 已有 `clinical-trials`（`search_trials`、`get_trial_details` 等）。

**改动**

- 在 `CapabilityGroup::Stats` 增加 `clinical-trials` 卡片：`GuidedChat`，**不绑 skill**，prompt 明确要求只用已启用的 `clinical-trials` MCP；未启用时点名 Settings → Connections 里要开的 connector，不要丢去空设置页。
- 仿 [caps.prompt.bio_db](../../../ui/src/i18n.rs) 的「最多五个问题、先问 PICO/病种/药物/NCT」。
- 中文标题建议：「临床试验注册检索」。blurb 写清：ClinicalTrials.gov；ChiCTR 本轮不做。

**验收**

- 点卡进入引导对话，不打开设置页（除非 connector 未启用且模型按 prompt 说明）。
- 窄测：catalog 含该 id，action 为 GuidedChat 且 `skill`/`specialist` 均为 None。

### 本工作流不做

- 不改 CapabilityGroup 枚举（仍 7 组）。
- 不把「我的文献库 / 知网」做成 `Action::None` 占位；等 C/D 就绪再用 `InstallThenGuided` 或 GuidedChat。

---

## 工作流 B：临床统计专家

**用户问题：** `nature-statistics` 审的是期刊写法；医生要的是选设计、算样本量、出 Table 1 / KM / Cox / ROC。

对标：[DoyoungJang/medsci-skills](https://github.com/DoyoungJang/medsci-skills) 的 `calc-sample-size` / `analyze-stats` / `design-study`。接入方式对齐现有 `data_cleaning` / `r_bioinformatics_figure`，**不要 vendoring 整包 22 个 skill**。

### B1. 内置 specialist `clinical_stats`

**改动**

- [src-tauri/src/specialists.rs](../../../src-tauri/src/specialists.rs)：新增 `CLINICAL_STATS_RUBRIC` + `builtin_clinical_stats()`，在 `ensure()` 里 materialize（与其它 builtin 一样：name/description/instructions 每次重钉）。
- 硬性规则（对齐数据清洗专家，避免问卷）：
  1. Intake 最多五个短问；每次一个。
  2. 先要数据文件/路径，或（仅样本量）要效应量/事件率/把握度。
  3. 禁止追问能从文件读出的行列 / schema。
  4. 不得捏造 n、p、HR、CI；缺包则给出安装命令再继续。
  5. 默认持久 `r` tool：`survival`、`tableone`、`pROC`、`broom`；倾向评分需要时再提示 `MatchIt`。
  6. 交付：可复现 R 脚本、结果表（项目相对路径）、`figures/*.svg`（+ PNG 仅供 `view_image`）、一段假设与限制。
- `skills`：可白名单 `nature-statistics`（写完后审核表述）+ `nature-figure`；不要绑整个 nature 家族。
- 单元测试：`ensure` 后能查到 `clinical_stats`；builtin 不可删除。

### B2. 首页卡片

- `CapabilityGroup::Stats` 增加 `clinical-stats`：`specialist_tile` 或 `specialist_skill_tile`，绑定 `clinical_stats`。
- 标题：「临床统计与研究设计」。blurb 写明：样本量、Table 1、生存、logistic、ROC；**不是**期刊统计措辞审核（那张仍是 `nature-statistics`）。
- 更新 Stats 组顺序测试：建议 `nature-statistics`、`clinical-stats`、`knowledge-graph`… 或把临床统计放在 `stats-analysis` 之前，避免两张「统计」卡看起来重复。

### B3. 可选薄 skill（若 rubric 不够）

仅当 specialist 指令无法稳定选对检验时，再加 `skills/clinical-stats/SKILL.md`：一张路由表（设计 → 检验 → R 包），**不要**复制 medsci 的协议/期刊匹配章节。

### 验收（手工 + 窄测）

- 合成或公开小表（如生存 2 组）：能产出 Table 1 + KM SVG + Cox 摘要，数字可从脚本重跑。
- 无数据、只给「HR=1.5，事件率 0.3，power 0.8」：能给出样本量与公式来源，不编造临床试验。
- WASM check + 相关 Playwright（点卡后侧栏标题为「临床统计与研究设计」）。

### 本工作流不做

- 不集成 ASReview，不做全自动 meta 成稿。
- 不接 SPSS GUI。
- 不在本 PR 扩展 PII → PHI。

---

## 工作流 C：Zotero + 本地 PDF RAG

**用户问题：** 能深读单篇 PDF，不能问「我自己库里这 200 篇的纳入标准怎么写」。

**接入原则：** 优先 MCP 插件，不重写 RAG。许可：PaperQA2 为 Apache-2.0；先核 zotero-mcp / zotero-research-assistant 的许可证再 vendoring。

### C1. 连接器文档与安装路径

**改动**

- [docs/basic-configuration.md](../../../docs/basic-configuration.md) 增加「我的文献库」：本机 Zotero 须开本地 API / 指定存储目录；推荐先装 [54yyyu/zotero-mcp](https://github.com/54yyyu/zotero-mcp) 读元数据与标注。
- 全文问答第二步：可选 [Future-House/paper-qa](https://github.com/Future-House/paper-qa) 或 [menyoung/paperqa-mcp-server](https://github.com/menyoung/paperqa-mcp-server)，指向 Zotero storage 或项目 `literature/`。
- 中英混合库可评估 [qiobn/zotero-research-assistant](https://github.com/qiobn/zotero-research-assistant)，须单独做许可与体积评估，不默认打进 DMG。

### C2. 首页卡片：`InstallThenGuided` 或「打开设置 → 插件」

两种实现，选一个，不要两张卡：

- **推荐：** `OpenSettings { section: "plugins" }` 或 `InstallThenGuided`，id `zotero-library`，组 `Assets`。未安装时说明要本机 Zotero + 插件；已安装则 GuidedChat，prompt 要求先用 Zotero MCP 列库再问答，禁止用模型记忆冒充库内论文。
- 不要把 PaperQA 打进默认 bundle（体积与 embedding 密钥）。

### C3. 安全

- 密钥走现有 secrets / Keyring，不进 SQLite。
- 测试不得依赖真实 Zotero 库：用假 MCP runner 或跳过网络的解析测试。
- 引用必须带回 Zotero key / 本地路径；答不出就说库里没有。

### 验收

- 无 Zotero：卡片说明缺什么，不假装已检索。
- 有 mock MCP：能列出 2–3 条题录并回答「哪篇提到纳入标准」，答案带来源 id。
- 文档写清：本机索引，不把整个 PDF 库上传云端（除非用户自己的 embedding API）。

---

## 工作流 D：知网 skill

**用户问题：** 中文综述、国自然立项依据、中华系列投稿离不开知网；现在只能让用户自己点「真实浏览器」。

**约束：** 知网无稳定官方 API。只复用已登录 Chrome 的浏览器桥（[docs/real-browser-automation.md](../../../docs/real-browser-automation.md) + `browser-use`）。禁止教用户绕过机构权限或批量抓站。

### D1. bundled skill `cnki-search`（薄路由，不内嵌爬虫）

**改动**

- 新增 `skills/cnki-search/SKILL.md`：
  - 先检测浏览器桥 / `browser-use` 是否就绪；未就绪则给开通步骤并停止。
  - 工作流：关键词或题名 → 结果列表（题名、作者、期刊、年、核心标记若页面可见）→ 用户点选 → 元数据 / 摘要 → 可选导出 GB/T 7714 或 `.ris` 到项目 `literature/`。
  - 下载 PDF/CAJ 仅当用户已有机构登录且明确要求；失败则说明权限，不重试打码。
- 可参考 [Qianxi-GXMU/cnki-skills](https://github.com/Qianxi-GXMU/cnki-skills) 的步骤拆分，**不要**原样 vendoring 若许可或 DevTools MCP 假设与本仓库浏览器桥不一致。
- [skills/THIRD_PARTY_LICENSES.md](../../../skills/THIRD_PARTY_LICENSES.md) 若引用了第三方步骤需记账。

### D2. 首页卡片

- `CapabilityGroup::Assets`：`skill_tile` id `cnki-search`，标题「知网中文文献」。
- Prompt：先确认浏览器桥；再问检索式（主题 / 作者 / 期刊 / 年份）；不要问语言偏好。

### D3. 测试

- 解析 / 路由测试：无浏览器时 skill 文本要求停止并提示开通。
- 禁止 CI 打真实 cnki.net。Playwright 只测「点卡后会话标题正确」。
- URL 过滤：知网域名应在浏览器 allow/prefer 列表（若 [browser_url_filters.rs](../../../src-tauri/src/browser_url_filters.rs) 有学术站点白名单，补上 `cnki.net` / `cnki.com.cn`）。

### 本工作流不做

- 万方 / 维普（可在 skill 里一句「本轮只走知网」）。
- 不把知网全文默认同步进云模型上下文；先落盘再按页读（`pdf-explore`）。

---

## 跨工作流测试与文档

每个行为 PR：

- `cargo fmt --all -- --check`（格式漂移则 `cargo fmt --all` 后单独提交）
- 相关 `cargo test`
- UI / Tauri：`cd ui && cargo check --target wasm32-unknown-unknown`；`ui-tests` 里补 Escape 与点卡冒烟
- 用户可见文案变更：更新 [docs/basic-configuration.md](../../../docs/basic-configuration.md)

## 明确不做（本 P0）

- 新 CapabilityGroup（如「临床研究」）
- 全自动 PRISMA 成稿 / 一键 meta
- 再堆 AF / Boltz / 单细胞模型
- 飞书微信、宠物、第二套能力 UI
- 把 `dmg的替身` 或无关文件写入仓库

## 建议 PR 切片

| PR | 内容 | 依赖 |
|---|---|---|
| 1 | A1 删除三张占位卡 | 无 |
| 2 | A2 露出 pdf-explore / 证据审读 / literature-review | 无 |
| 3 | A3 临床试验注册检索卡 | 无（MCP 已在） |
| 4 | B1+B2 临床统计专家 + 首页卡 | 无 |
| 5 | C 文献库卡片 + 文档 + 插件安装说明 | 可与 4 并行 |
| 6 | D 知网 skill + 首页卡 + URL 过滤 | 浏览器桥已存在 |

PR 1–3 应在一周内可合并；4–6 各自独立验收后再考虑 P1。
