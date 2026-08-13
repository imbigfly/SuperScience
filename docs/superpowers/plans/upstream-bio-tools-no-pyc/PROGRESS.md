# bundled bio-tools 禁止写入 pyc：执行进度

## 固定边界

- 工作树：`<dedicated-worktree>/wispscience`（任务指定的隔离 checkout）
- 分支：`codex/upstream-bio-tools-no-pyc`
- 基线：`origin/main=bdc4822b3a91d3b42c4c4f388047181725cb3529`
- 仅修改 `crates/wisp-mcp/src/client.rs`、本目录进度文档；不改签名/构建配置，不操作 App bundle、用户数据或其他 checkout。

## 机械验收

- 回归测试证明 bundled bio-tools child env 的 `PYTHONDONTWRITEBYTECODE` 恒为 `1`，调用方传入 `0` 也必须被覆盖。
- 测试先在旧实现上失败，再以最小代码修改通过。
- 依次通过定向测试、`cargo test -p wisp-mcp`、`cargo fmt --all -- --check`、`cargo test --workspace`、`cargo run -p wisp-mcp --example smoke`。
- 仅提交范围文件，push 到 fork，并向 `xuzhougeng/wisp-science:main` 创建小 PR；持续跟进 CI 至绿或确认原始外部阻塞。

## 状态

- [x] Task 1/8：核对固定 worktree、branch、HEAD、clean 与本地 `origin/main` tracking ref；均为指定 SHA，工作树初始 clean。
- [x] Task 2/8：检索已有 issue/PR；精确关键词未发现同一 bug 或修复。
- [x] Task 3/8：添加 child env 强制覆盖的回归测试并保留红测证据。
- [x] Task 4/8：最小修复并通过定向测试。
- [x] Task 5/8：运行 crate / fmt / workspace / MCP smoke 验证。
- [x] Task 6/8：复核 diff 与 staged 范围；仅有 1 个源码文件与 2 个任务记录文件，`git diff --check` 通过。
- [ ] Task 7/8：push 并创建官方 PR。
- [ ] Task 8/8：跟进 CI 至全绿或记录外部阻塞。

## 现场证据

- 初始 `git status -sb`：`## codex/upstream-bio-tools-no-pyc...origin/main`，无修改。
- 初始 `HEAD` / 本地 `origin/main`：均为 `bdc4822b3a91d3b42c4c4f388047181725cb3529`。
- 总控在创建本任务前已于主 repo 成功执行 `git fetch origin`（exit 0），随后确认 live `origin/main` 仍为上述固定 SHA；本线程额外网络探针失败不影响该基线结论。
- 重复项：GitHub Search API 对 `PYTHONDONTWRITEBYTECODE`、`bytecode`、`bundle seal` 返回 0；`__pycache__` 与 `code signature` 的少量命中均与本问题无关。
- 红测：`client::tests::bio_tools_command_forces_python_not_to_write_bytecode` 执行 1 项并失败，实际 `Some("0")`、期望 `Some("1")`。
- 最小修复：caller env 应用后追加 `cmd.env("PYTHONDONTWRITEBYTECODE", "1")`；同一测试转绿（1 passed）。
- `cargo test -p wisp-mcp`：6 passed，0 failed；doc-test 1 ignored。
- `cargo fmt --all -- --check`：PASS，无输出。
- `cargo test --workspace`：最终在本任务外接卷临时 target 中 PASS；1,037 passed，0 failed，1 ignored，另有 `wisp-acp process tests passed`。
- `cargo run -p wisp-mcp --example smoke`：PASS；mock `echo` tool round-trip 成功并输出 `wisp-mcp smoke OK`。
- 磁盘处理：首次 worktree-local target 编译因 `errno=28` 中止；仅清理本任务新生成的 3.0 GiB `target/`。共享 target 复跑仍因空间中止且未清理共享内容。最终在外接卷专用临时 target 完成全套验证，并删除该 2.8 GiB 可重建临时目录。

## 下一步

提交后向 fork push，并创建官方 PR。
