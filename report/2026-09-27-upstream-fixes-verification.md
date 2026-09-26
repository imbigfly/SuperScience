# 上游修复摘取 · 功能测试报告

- **日期**：2026-09-27
- **分支**：`local/brand-tiancheng`（已合并 `local/upstream-fixes-202609`，合并后顶端 `92018404`）
- **背景**：从上游 v1.12.0–v1.15.0 选择性摘取 15 个孤立 bug 修复（详见 [2026-09-26-upstream-fixes.md](2026-09-26-upstream-fixes.md)），不引入版本功能、不改数据库、不升级依赖、品牌化完整保留。

## 一、修改了哪些功能（用户视角）

| # | 功能点 | 修复前行为 | 修复后行为 |
|---|---|---|---|
| 1 | **图片输入自动缩放**（#1235） | 长边超 2048px 的图片按大文件流程暂停询问，或直接发送超限图 | 附件与图片读取先解码校验真实格式；长边超 2048px 自动生成按比例 JPEG 输入副本（源文件不动，模型可见标签附缩放提示）；损坏图片本地拒绝 |
| 2 | **新窗口会话水合**（#1237） | 新窗口打开项目时运行中的会话显示空白、待审批项丢失；迟到的审批快照会把已处理的审批"复活" | 运行中会话与待审批实时恢复；审批按 `approval_id` 归属校验，迟到快照不再覆盖；`confirm-resolved` 事件同步各窗口 |
| 3 | **模型 token 输入焦点**（#1246） | 编辑模型 token 上限时输入框被重挂载，焦点丢失 | 媒体类型判断提为 memo，输入期间 DOM 节点稳定，焦点保持 |
| 4 | **动画界面稳定性**（#1249） | 同值信号写入导致右侧栏/中央预览反复重挂载（Windows 上回复后首次点击明显） | 同值写入不再触发重挂载 |
| 5 | **网络错误提示**（#1255） | 连接失败时提示指向已移除的"模型 API 代理"设置 | 新增操作系统系统代理检测（`system_proxy.rs`），错误信息点名实际生效的代理（配置/环境变量/系统代理），指向 设置 → 常规 → 网络 → 直连 |
| 6 | **Windows 首击不闪屏**（#1258） | 回复完成后首次点击会拆掉已显示的会话外壳重建（页面闪烁） | 屏幕上已有项目的 `open-session` 只切换会话，不重建外壳 |
| 7 | **分支保留来源文件夹**（#1260） | 创建会话分支时丢失来源文件夹归属 | 分支继承来源文件夹；旧库缺列自动补齐 |
| 8 | **分支随文件夹移动**（#1268） | 会话移动文件夹后分支后代关系错乱 | 移动会话时持久化整棵分支子树一起迁移 |
| 9 | **SSH 版本报错**（#1269） | 本地 OpenSSH 低于 8.4 时表现为连接超时，难以诊断 | 探测前明确报"OpenSSH 过旧"，解析实际版本号显示 |
| 10 | **建议文件名不再误判**（#1270） | 模型建议的文件名（如 `analysis.py`）被当作工作区路径渲染成可点击链接 | 仅真实存在于项目根下的路径才成为链接 |
| 11 | **auto-review 会话隔离**（#1293） | 在一个会话开启自动审核会影响下一个新会话 | 会话级开关互不影响；设置页只改"新会话默认" |
| 12 | **源码编辑器平移**（#1305） | 居中源码编辑器水平方向无法平移（长行看不全） | 修复水平滚动；插入符与高亮的文本度量对齐 |
| 13 | **figure QA 裁剪图目录**（#1313） | figure-style 质检裁剪图混入项目输出目录 | 裁剪图移入 `.cache/`，输出目录保持干净 |
| 14 | **分支合并摘要预算**（#1318） | 合并摘要用硬编码输出上限，长上下文摘要被截断 | 按 profile 配置与模型目录上限取输出预算 |
| 15 | **失效路径不可点击**（#1321） | 聊天中提到的文件删除后路径仍可点击，打开破损预览 | `WorkspacePathLiveness` 存在性检查：已删除文件路径自动还原为普通代码文本 |
| 16 | **命令面板方向键**（#1332） | 命令面板方向键导航会把选中项滚出可视窗口 | 分组渲染 + 仅在边缘滚动，选中项始终可见 |
| 17 | **退出序列日志**（#1359） | App 退出卡死时日志静默，无法定位卡在哪一步 | 退出序列逐步记录 `app.exit.step`（本地基线无 MCP broker，相应两步跳过） |

## 二、测试结论

### 2.1 全量自动化（合并后分支）

| 套件 | 结果 |
|---|---|
| `cargo fmt --all -- --check` | **通过** |
| `cargo check --workspace` | **通过** |
| `cargo test --workspace` | **全部通过**（约 2200 项，0 失败，含 963 项 UI 契约与 286 项 store 等大套件） |
| `cd ui && cargo check --target wasm32-unknown-unknown` | **通过** |
| `cd ui-tests && npm ci && npx playwright test` | **650 通过 / 3 跳过 / 0 失败**（26.7 分钟） |

### 2.2 按修复逐项的针对性测试（2026-09-27 复核）

| 功能点 | 针对性测试 | 结果 |
|---|---|---|
| #1235 图片缩放 | `superscience-tools` image ×5（含 9052px 边自动缩放、源文件不变）+ `superscience-tauri` image_helper ×1 | **6/6 通过** |
| #1237 会话水合 | `superscience-tauri` hydration ×1 + Playwright `session-hydration.spec.ts` ×7（含"迟到快照不复活审批""另一窗口解决审批移除卡片""失败水合显示重试"） | **8/8 通过** |
| #1246 token 焦点 | Playwright 模型表单子集（32 项针对性运行内含） | **通过** |
| #1249 界面稳定性 | Playwright "same-value writes never remount the right pane or center preview" | **通过** |
| #1255 网络提示 | `superscience-llm` system_proxy ×2 + Playwright "a leftover proxy connect error points at General Network settings" + settings-layout 子集 | **通过** |
| #1258 首击不闪屏 | Playwright "open-session for the project already on screen switches conversations without rebuilding the shell" | **通过** |
| #1260 分支来源 | `superscience-store` branch ×9（含"分支创建时继承来源文件夹""旧库自动补列"） | **9/9 通过** |
| #1268 分支随移动 | 同上套件含 "moving_session_moves_persisted_branch_subtree" | **通过** |
| #1269 SSH 版本 | `superscience-runs` openssh ×5（banner 解析、8.4 最低版本、探测前拒绝） | **5/5 通过** |
| #1270 文件名误判 | Playwright 现有 workspace 路径测试（#1270 扩展断言，含于全量 650 项） | **通过** |
| #1293 auto-review 隔离 | `superscience-tauri` auto_review ×2（默认关闭、修改持久化、按会话作用域） | **2/2 通过** |
| #1305 编辑器平移 | Playwright `editor-alignment.spec.ts` ×2 | **通过**（注：独立运行时偶发 1 项字体度量断言 flaky，复跑两轮均全过；与平移改动无关，详见 2.3） |
| #1313 QA 裁剪目录 | `python3 python/test_skill_sidecars.py` ×12 | **12/12 通过** |
| #1318 合并摘要预算 | `superscience-tauri` branch_summary ×2（含核心断言"output budget follows the profile up to the model ceiling"） | **2/2 通过** |
| #1321 失效路径 | Playwright workspace 路径子集 + 全量 650 项内含 | **通过** |
| #1332 命令面板 | Playwright "command palette arrows move inside the window and scroll only at the edge" | **通过** |
| #1359 退出日志 | 编译验证 + 退出路径日志埋点（无行为断言型测试） | **通过** |

### 2.3 已知事项

- `editor-alignment.spec.ts` 的 "(custom)" 用例在**单独**运行时约 1/4 概率出现字体度量时序 flaky（期望 16px 读到 12px，即自定义字号尚未应用）。全量运行与复跑均通过，属测试自身时序敏感，与本次修复无关；如再现可在测试中增加字号应用等待。
- Playwright 运行后会刷新 `docs/design-qa/` 下两张对比截图（工作区污染），已还原，不影响基线。
- #1237 的 limiter cancel 子行为、#1235 的 MCP rich-results 部分、#1359 的 MCP broker 关停步骤因依赖本地不存在的 v1.12 架构未带入（记录于摘取报告）。

## 三、兼容性结论

- **数据库零改动**：无新迁移，v1.11.0 数据完整兼容，可随时回退（回退点：合并前 `local/brand-tiancheng` = `8129e078`）。
- **依赖新增仅一处**：`superscience-llm` 平台依赖 `system-configuration`（macOS）/`windows-registry`（Windows），用于系统代理检测；tao 等核心依赖版本未动。
- **品牌化完好**：全量 Playwright 含品牌相关用例（wordmark、branded login card 等）均通过。

## 四、产物

本地 DMG 打包结果见同日打包记录（`scripts/package-macos-local.sh`）。
