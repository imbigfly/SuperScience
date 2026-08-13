# bundled bio-tools 禁止写入 pyc：阻塞项

## 当前阻塞（不影响本地实现）

1. 额外网络探针：本线程 `git ls-remote` 因本机 HTTPS proxy `127.0.0.1:7897` 不可达而失败；绕过 proxy 时当前执行环境也无法直连 GitHub。总控已在任务创建前成功 fetch 并确认 live `origin/main` 等于固定 SHA，因此这不是基线未知，也不阻塞实现。
2. GitHub 发布认证：`gh 2.96.0` 已安装，但 `gh auth status` 报默认账户 token 无效。代码与本地验证继续；发布时先向 fork push，再由可用 GitHub connector 创建上游 PR；不因该项停下代码验证。

## 已解决

- 主磁盘空间不足：worktree-local 与共享 target 两次 workspace 编译均在执行测试前因 `errno=28` 中止。未清理共享 target；改用外接卷本任务专用临时 target 后，完整 workspace 与 MCP smoke 均通过，随后删除该临时目录。

## 已排除

- 未发现相同 issue/PR；无重复实现阻塞。
