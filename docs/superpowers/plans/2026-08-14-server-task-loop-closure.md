# 服务器任务闭环实施计划：产物取回与远端清理

> 设计原则：**任务属于项目，产物属于项目，服务器只负责计算。** 服务器是可随时丢弃的
> 执行环境，不是文件中心。

## 现状与缺口

生命周期前中段已闭环：`runs` 表 + `RunManager`（`src-tauri/src/run_context/`）提供
detached 执行（local/WSL/SSH-direct）、状态机、生命周期租约、重启后 reconciler 重连、
`monitor_run`/`cancel_run` 工具、`transfer_between_contexts` 传输进度、本地 harvest 与
Artifact 溯源。

断点集中在尾段：

1. **远端产物取回缺失**：SSH run 的 `output_specs` 只接受显式 `ssh://` URI 并登记为
   引用（`run_context.rs` 中显式拒绝远端 glob），没有"成功后按 glob 拉回并注册为本地
   ArtifactVersion"的路径。
2. **远端工作目录清理完全没有**：`remote_workdir`（`.wisp-science/runs/<id>/`）只记录
   从不删除，没有工具、GC 或保留策略。
3. **撤回文件无概念**：上传（run inputs staging、`transfer_between_contexts`）不留
   登记，无法回答"哪些远端文件已无人引用、可以删"。
4. **无顺序保障**：没有 `harvested_at`/`cleaned_at`，无法约束"先确认取回、再允许清理"。

## 交付策略

按小 PR 交付，每个 PR 一个持久抽象，各自带迁移、工具、测试。PR 1→2→3 有依赖顺序
（清理必须以取回确认为前置；撤回清理复用清理执行路径）；PR 4 是收尾策略层，可独立。

所有测试禁止真实 SSH/网络：远端行为用现有 `RunCommandRunner` fake、临时目录和
mocked Tauri 命令覆盖。迁移同时改 `crates/wisp-store/migrations/`（新编号文件）和
`crates/wisp-store/src/lib.rs` 的幂等补列，保持向后兼容。

## PR 1：Harvest v2 —— 远端产物枚举与取回

**用户问题：** SSH 任务成功后，最终结果必须能自动回到项目本地并注册为带 checksum 的
ArtifactVersion，而不是永远以 `ssh://` 引用留在服务器上。

### 设计

- 成功收尾时（`finish_remote_run`），对 SSH run 的非 `ssh://` `output_specs` 执行远端
  收集脚本（复用 `ssh_script_command` 的 `sh -s` 通道）：在 `remote_workdir` 内按 glob
  匹配文件，输出 `相对路径\t字节数\tsha256`（`sha256sum`，缺失时 `shasum -a 256`），并把
  匹配文件按相对路径硬链/复制进 `<workdir>/harvest/`。
- 单次 `scp -r` 把 `harvest/` 拉到项目本地私有暂存目录（`.wisp/harvest/<run_id>/`），
  逐文件校验 sha256，失败则整体报错、不注册。
- 对暂存目录复用现有 `harvest_run_outputs`（`src-tauri/src/harvest.rs`），走既有的
  snapshot/reference/lineage 逻辑；`source_path` 记录远端相对路径。
- 尊重大数据规则：`residency: remote`、超过 `max_file_mb`/`max_total_mb` 的文件不下载，
  由远端脚本返回的 size/checksum 直接注册为 `ssh://<alias>/<abs path>` 外部引用
  （补上现在缺失的 checksum 和 size）。
- 移除 `run_context.rs` 中"SSH direct output_specs must be explicit ssh:// references"
  的拒绝分支；`ssh://` URI spec 保持原语义。
- 迁移：`runs` 增加 nullable `harvested_at`（INTEGER）。本地/WSL run 在既有 harvest 成功
  后同样写入。harvest 失败不改变 run 终态，错误记入 `last_poll_error`（沿用现状），
  `harvested_at` 保持 NULL，作为 PR 2 清理的硬前置。
- 新增 `harvest_run` 工具/Tauri 命令：对 `succeeded` 且 `harvested_at IS NULL` 的 run
  手动重试取回（自动 harvest 失败后的恢复路径，也覆盖旧数据）。

### 接口

```rust
// runs 表新列
runs.harvested_at: Option<i64>

// src-tauri/src/run_context/remote.rs
fn remote_collect_script(workdir: &str, specs: &[OutputSpec]) -> String;
fn parse_collect_manifest(stdout: &str) -> Result<Vec<RemoteOutputEntry>, String>;

// src-tauri/src/harvest.rs（签名不变，新增远端入口）
async fn harvest_remote_run_outputs(store, runner, run, specs) -> Result<Vec<HarvestedArtifact>, String>;

// 工具
harvest_run { run_id }
```

### 测试

- fake runner：glob 命中多文件 → manifest 解析、checksum 校验、本地注册、
  `run_outputs`/`produced` 边、`harvested_at` 写入。
- checksum 不匹配 → 报错、不注册、`harvested_at` 为 NULL、run 仍 `succeeded`。
- 超限文件 → 注册为 `ssh://` 引用且带 size/checksum，不下载。
- `logical_key` 多文件命中仍报错（与本地一致）。
- `ssh://` URI spec 行为不回归；本地/WSL run 写入 `harvested_at`。
- legacy 库重开幂等补列。

### 验证

```bash
cargo test -p wisp-science-desktop harvest
cargo test -p wisp-store runs
cargo fmt --all -- --check
```

## PR 2：Run 工作区清理 —— cleanup_run_workspace

**用户问题：** 任务结束后，远端 `inputs/`、日志、supervisor 文件和中间产物应能安全
删除，且绝不能在产物取回确认之前删。

### 设计

- 迁移：`runs` 增加 nullable `cleaned_at`（INTEGER）、`cleanup_error`（TEXT）。
- 新增 `cleanup_run_workspace` 工具 + Tauri 命令。前置条件（Store 层校验，不信任
  调用方）：
  - run 处于终态（`succeeded`/`failed`/`cancelled`/`timed_out`/`lost`）；
  - `succeeded` 且 `output_specs` 非空时要求 `harvested_at IS NOT NULL`；
  - 未清理过（`cleaned_at IS NULL`），重复调用幂等返回。
- 执行：SSH 走 `sh -s` 通道 `rm -rf`；local/WSL 走对应 transport 删除（Windows 用
  原生删除，不假设 POSIX）。**路径安全**：只删除 handle 中记录的 workdir，且必须
  匹配 `.wisp-science/runs/<run_id>` 模式，拒绝任何其他路径；不展开来自远端的字符串。
- 删除失败写 `cleanup_error` 并保留 `cleaned_at` 为 NULL，可重试；成功写 `cleaned_at`
  并清空 `cleanup_error`。
- `lost` 状态的 run（进程身份无法确认）清理前先做一次 kill-by-token 兜底，避免删除
  仍在写入的目录。
- UI：run 详情/RunMonitorCard 增加"清理服务器文件"操作与已清理状态显示（图标走
  `compose_icon()` 新 kind）；`get_run_detail`/`list_runs` DTO 带出新字段。
- Agent 提示：`monitor_run` 成功返回文案中提示可清理（不自动清理，自动化留给 PR 4）。

### 接口

```rust
runs.cleaned_at: Option<i64>
runs.cleanup_error: Option<String>

Store::mark_run_cleaned(run_id, owner) -> Result<bool>
Store::record_run_cleanup_error(run_id, error)

// 工具 / Tauri 命令
cleanup_run_workspace { run_id }
```

### 测试

- 前置条件矩阵：running 拒绝；succeeded+specs+未 harvest 拒绝；harvest 后允许；
  failed/cancelled 直接允许；二次调用幂等。
- fake runner 断言下发的删除命令路径被约束在 `.wisp-science/runs/<id>`；恶意
  workdir（`~`、`/`、含 `..`）被拒绝。
- 删除失败 → `cleanup_error` 落库、可重试；成功 → `cleaned_at` 落库。
- Windows 本地路径删除逻辑单测（不依赖 POSIX）。
- UI Playwright：run 卡片显示清理按钮 → 点击 → 状态更新；Escape 栈规则不回归。

### 验证

```bash
cargo test -p wisp-science-desktop cleanup
cd ui && cargo check --target wasm32-unknown-unknown
cd ../ui-tests && npm ci && npx playwright test
```

## PR 3：远端 staging 登记 —— 撤回与孤儿文件清理

**用户问题：** 上传到服务器但随后被撤回、替换或不再使用的文件必须可见、可清理，
使"直接丢弃这台服务器"成为可验证的操作。

### 设计

- 迁移：新表 `remote_staging`：

```sql
CREATE TABLE IF NOT EXISTS remote_staging (
    id          TEXT PRIMARY KEY,
    project_id  TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    context_id  TEXT NOT NULL,
    run_id      TEXT,              -- 所属 run；transfer 类 run 也填
    remote_path TEXT NOT NULL,     -- 远端绝对/HOME 相对路径
    source      TEXT NOT NULL,     -- 'run_input' | 'transfer'
    checksum    TEXT,
    size_bytes  INTEGER,
    created_at  INTEGER NOT NULL,
    removed_at  INTEGER
);
CREATE INDEX IF NOT EXISTS ix_remote_staging_ctx ON remote_staging(context_id, removed_at);
```

- 写入点：SSH run inputs staging（`stage_inputs` 成功后逐文件登记）、
  `transfer_between_contexts` 上传成功后登记目标路径。run workdir 内的文件在 PR 2
  清理成功时批量标记 `removed_at`。
- 新增 `list_remote_files` 工具/Tauri 命令：按 context 列出本项目登记的未移除远端
  文件，标注归属 run 与其状态，区分"活跃引用"（run 未终态或未清理）与"孤儿"
  （run 已终态且已清理/不存在，或 transfer 目标已被更新版本替代——同一
  `remote_path` 存在更晚的登记）。
- 新增 `remove_remote_files` 工具：删除指定孤儿条目对应的远端文件（复用 PR 2 的
  安全删除通道；只允许删除登记在册的路径），成功标记 `removed_at`。活跃引用拒绝
  删除，除非显式 `force`。
- UI：Environment 右栏每个 SSH context 增加"远端文件"视图（登记列表、孤儿标记、
  清理操作）。

### 接口

```rust
Store::record_remote_staging(entry)
Store::list_remote_staging(context_id, include_removed)
Store::mark_remote_staging_removed(ids)

list_remote_files { context_id }
remove_remote_files { context_id, ids, force? }
```

### 测试

- run inputs staging / transfer 上传均产生登记；relay 中转不登记本地临时路径。
- 同一 `remote_path` 二次上传 → 旧条目判定为"被替换"孤儿。
- 活跃 run 引用的条目拒绝删除；`force` 可删；删除后 `removed_at` 落库。
- 未登记路径的删除请求被拒绝（防任意远端删除）。
- PR 2 清理联动：workdir 清理后其 inputs 条目自动标记移除。
- legacy 库幂等建表。

### 验证

```bash
cargo test -p wisp-store remote_staging
cargo test -p wisp-science-desktop remote_files
cd ui && cargo check --target wasm32-unknown-unknown
cd ../ui-tests && npx playwright test
```

## PR 4：保留策略 —— 成功任务自动清理（收尾，可选）

**用户问题：** 用户不应手动清理每个任务；成功且已取回的任务应按项目策略自动回收
远端空间。

### 设计

- 项目级设置 `run_workspace_retention_days`（默认 NULL=不自动清理，显式 opt-in）。
- 复用 `RunManager` 的后台 poller/reconciler 周期：扫描
  `succeeded && harvested_at IS NOT NULL && cleaned_at IS NULL && ended_at < now - N days`
  的 run，逐个走 PR 2 的 `cleanup_run_workspace` 路径（含全部前置校验与路径约束）。
- 清理动作写入 run 时间线（`cleanup_error`/`cleaned_at` 已有），UI 设置页暴露开关。
- 文档：更新 `skills/remote-compute-ssh/SKILL.md` 与 control-plane spec，写明生命周期
  九段全部落地：上传 → 创建 → 后台执行 → 状态 → 日志 → 取消/重连 → 登记 → 取回 → 清理。

### 测试

- 到期 run 被自动清理、未到期/未 harvest/失败态不动。
- 清理失败不阻塞 poller，错误落库且下轮重试。
- 设置默认关闭；开启/关闭即时生效。

### 验证

```bash
cargo test -p wisp-science-desktop retention
cargo test --workspace
cargo fmt --all -- --check
```

## 风险与限制

- **远端工具依赖**：收集脚本依赖 `sha256sum`/`shasum`、`tar` 或硬链支持；沿用 SSH
  runner 既有的 preflight 模式显式探测并给出可读错误，不静默降级。
- **大量小文件**：单次 `scp -r` 拉整个 `harvest/` 目录避免每文件一次连接；超大目录
  的性能优化（tar 流式）留作后续，不阻塞正确性。
- **调度器（SLURM）后端**：不在本计划内；清理/取回抽象以 workdir 为单位，未来调度
  器 run 可复用。
- **不做**：`transfers` 独立表（沿用 `kind='file_transfer'` 的 run）、`data_assets`
  首类表（research_nodes 现状够用）、全项目 runs timeline 页面（另行立项）。
