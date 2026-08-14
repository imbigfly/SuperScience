<div align="center">

<img src="docs/assets/logo.svg" alt="天成科研助手 Science logo" width="128" />

# 天成科研助手 Science

**开源、本地优先的 AI 科研工作台。**

WISP — *Workspace for Intelligent Scientific Practice*
（面向智能科研实践的工作空间）

<a href="https://github.com/imbigfly/SuperScience/releases"><img src="https://img.shields.io/github/v/release/imbigfly/SuperScience" alt="Release"></a>
<a href="https://github.com/imbigfly/SuperScience/releases"><img src="https://img.shields.io/github/downloads/imbigfly/SuperScience/total" alt="下载量"></a>
<a href="https://doi.org/10.5281/zenodo.21869520"><img src="https://zenodo.org/badge/1285857639.svg" alt="DOI"></a>
<a href="https://github.com/imbigfly/SuperScience/blob/main/LICENSE"><img src="https://img.shields.io/github/license/imbigfly/SuperScience" alt="许可证"></a>
<a href="https://github.com/imbigfly/SuperScience/stargazers"><img src="https://img.shields.io/github/stars/imbigfly/SuperScience?style=social" alt="Stars"></a>
<br>
<a href="https://github.com/imbigfly/SuperScience/releases"><img src="https://img.shields.io/badge/Windows-supported-0078D4" alt="支持 Windows"></a>
<a href="https://github.com/imbigfly/SuperScience/releases"><img src="https://img.shields.io/badge/macOS-supported-000000" alt="支持 macOS"></a>
<a href="https://github.com/imbigfly/SuperScience/releases"><img src="https://img.shields.io/badge/Linux-supported-FCC624" alt="支持 Linux"></a>

[English](README.md) · [简体中文](README_zh.md) · [网站](https://xuzhougeng.github.io/天成科研助手/) · [Releases](https://github.com/imbigfly/SuperScience/releases)

<img src="docs/assets/app-home.png" alt="天成科研助手 Science 桌面应用正在运行内置的 RNA-seq 分析演示" width="100%" />

</div>

检索文献、运行 Python 与 R、查询约 80 个科学数据库，把图、Run、判断和稿件
留在同一个项目里。数据、会话和凭据都在你自己的机器上。

模型你自己选。数据留在本地。

## 你可以用它做什么

**能真正干活的 Agent**

接入 OpenAI 兼容或 Anthropic 模型，也可以通过 ACP 驱动 Codex / Claude Code。
Agent 读写项目文件、执行 shell，并按需加载 Skills（`SKILL.md`），不会把目录
塞进提示词。默认走审批门控，需要时再开 Full Permission。

**从笔记本到远程服务器**

持久化 Python / R 内核，变量跨 cell、会话和重启保留。本地、WSL、SSH 主机注册
一次即可探测硬件、提交带实时日志的长 **Run**。密钥存本地 secrets 文件，不进 SQLite。

**为科研而生**

通过内置 MCP 访问 PubMed、GEO 等约 80 个数据库。离线预览 notebook、PDF、Office
和图片。[探索分支](docs/exploration-branches.zh-CN.md)让你试一条方向而不改主线。
[出版工作区](docs/publication-evidence.md)把稿件修订冻成可验证的证据胶囊。

**会记忆的工作台**

重启后完整历史还在。一键撤销某一轮的文件改动。`@` 附加产物与运行时，`#` 检索
已保存会话，`/` 套用 skill。[加密手动同步](docs/project-sync.zh-CN.md)与
[项目迁移](docs/project-transfer.md)——绝不在后台偷跑。

## 开始使用

1. 从 [GitHub Releases](https://github.com/imbigfly/SuperScience/releases) 下载。
2. 打开内置演示（无需 API Key），看完整的 RNA-seq 轨迹。
3. 在 **设置 → 模型** 中添加模型，然后开一个项目。

| 平台 | 安装包 |
|------|--------|
| Windows | 已签名 MSI / NSIS |
| macOS | 已签名并公证的 `.dmg`（Apple Silicon + Intel） |
| Linux | `.deb` / AppImage（x86_64 + aarch64） |

上手教程：[基础配置](docs/basic-configuration.md) ·
[模型配置](docs/model-configuration.md) ·
[ACP Agents](docs/acp-agents.md)

源码构建、CLI 与架构见[开发指南](docs/development.md)。

## 文档

| | |
|---|---|
| **上手** | [基础配置](docs/basic-configuration.md) · [模型](docs/model-configuration.md) · [ACP](docs/acp-agents.md) |
| **科研** | [探索分支](docs/exploration-branches.zh-CN.md) · [证据胶囊](docs/publication-evidence.md) · [选题库](docs/case-studies.zh-CN.md) |
| **项目** | [迁移](docs/project-transfer.md) · [同步](docs/project-sync.zh-CN.md) · [全局库](docs/global-library.md) |
| **算力** | [终端](docs/terminal-sessions.md) · [远程文件](docs/remote-file-browser.md) · [传输](docs/server-transfers.md) |
| **扩展** | [Skills](docs/skills.md) · [插件](docs/feature-plugins.md) · [委派](docs/agent-delegation.md) · [IM](docs/channels.md) · [浏览器](docs/real-browser-automation.md) |
| **开发** | [开发指南](docs/development.md) · [无头评测](docs/headless-agent-testing.md) |

## 社区

感谢每一位提 issue、发 PR、以及把 天成科研助手 用在真实课题上的人。

<p>
  <a href="https://github.com/Yu-Qiao-sjtu"><img src="https://avatars.githubusercontent.com/u/88706761?v=4&amp;s=96" width="64" height="64" alt="@Yu-Qiao-sjtu" title="@Yu-Qiao-sjtu"></a>
  <a href="https://github.com/lfz0924"><img src="https://avatars.githubusercontent.com/u/82395287?v=4&amp;s=96" width="64" height="64" alt="@lfz0924" title="@lfz0924"></a>
  <a href="https://github.com/jarxunlai"><img src="https://avatars.githubusercontent.com/u/199478724?v=4&amp;s=96" width="64" height="64" alt="@jarxunlai" title="@jarxunlai"></a>
  <a href="https://github.com/OrigamiSheep"><img src="https://avatars.githubusercontent.com/u/48906039?v=4&amp;s=96" width="64" height="64" alt="@OrigamiSheep" title="@OrigamiSheep"></a>
  <a href="https://github.com/LeeJyee"><img src="https://avatars.githubusercontent.com/u/166231040?v=4&amp;s=96" width="64" height="64" alt="@LeeJyee" title="@LeeJyee"></a>
  <a href="https://github.com/stardustFFF"><img src="https://avatars.githubusercontent.com/u/306053694?v=4&amp;s=96" width="64" height="64" alt="@stardustFFF" title="@stardustFFF"></a>
  <a href="https://github.com/Doctorluka"><img src="https://avatars.githubusercontent.com/u/101385826?v=4&amp;s=96" width="64" height="64" alt="@Doctorluka" title="@Doctorluka"></a>
  <a href="https://github.com/Charlesyu153"><img src="https://avatars.githubusercontent.com/u/232734740?v=4&amp;s=96" width="64" height="64" alt="@Charlesyu153" title="@Charlesyu153"></a>
  <a href="https://github.com/xiaowen621"><img src="https://avatars.githubusercontent.com/u/241900839?v=4&amp;s=96" width="64" height="64" alt="@xiaowen621" title="@xiaowen621"></a>
  <a href="https://github.com/liaoyuan919"><img src="https://avatars.githubusercontent.com/u/240658511?v=4&amp;s=96" width="64" height="64" alt="@liaoyuan919" title="@liaoyuan919"></a>
  <a href="https://github.com/lhx-JIPS"><img src="https://avatars.githubusercontent.com/u/33241642?v=4&amp;s=96" width="64" height="64" alt="@lhx-JIPS" title="@lhx-JIPS"></a>
  <a href="https://github.com/chenzhiyu48"><img src="https://avatars.githubusercontent.com/u/65606400?v=4&amp;s=96" width="64" height="64" alt="@chenzhiyu48" title="@chenzhiyu48"></a>
  <a href="https://github.com/liuyc414"><img src="https://avatars.githubusercontent.com/u/190511200?v=4&amp;s=96" width="64" height="64" alt="@liuyc414" title="@liuyc414"></a>
  <a href="https://github.com/kevinzzzhang76-dot"><img src="https://avatars.githubusercontent.com/u/251931886?v=4&amp;s=96" width="64" height="64" alt="@kevinzzzhang76-dot" title="@kevinzzzhang76-dot"></a>
  <a href="https://github.com/Shawn-Gua"><img src="https://avatars.githubusercontent.com/u/110019576?v=4&amp;s=96" width="64" height="64" alt="@Shawn-Gua" title="@Shawn-Gua"></a>
  <a href="https://github.com/Hayesss"><img src="https://avatars.githubusercontent.com/u/66942436?v=4&amp;s=96" width="64" height="64" alt="@Hayesss" title="@Hayesss"></a>
  <a href="https://github.com/Az-Fan"><img src="https://avatars.githubusercontent.com/u/189823792?v=4&amp;s=96" width="64" height="64" alt="@Az-Fan" title="@Az-Fan"></a>
  <a href="https://github.com/19951219asd"><img src="https://avatars.githubusercontent.com/u/118892832?v=4&amp;s=96" width="64" height="64" alt="@19951219asd" title="@19951219asd"></a>
  <a href="https://github.com/yeshubiao2017-source"><img src="https://avatars.githubusercontent.com/u/233231577?v=4&amp;s=96" width="64" height="64" alt="@yeshubiao2017-source" title="@yeshubiao2017-source"></a>
  <a href="https://github.com/xuxh95"><img src="https://avatars.githubusercontent.com/u/299415390?v=4&amp;s=96" width="64" height="64" alt="@xuxh95" title="@xuxh95"></a>
  <a href="https://github.com/xiaoshen19930901"><img src="https://avatars.githubusercontent.com/u/24424905?v=4&amp;s=96" width="64" height="64" alt="@xiaoshen19930901" title="@xiaoshen19930901"></a>
  <a href="https://github.com/scsksprings"><img src="https://avatars.githubusercontent.com/u/60927616?v=4&amp;s=96" width="64" height="64" alt="@scsksprings" title="@scsksprings"></a>
  <a href="https://github.com/lpc520"><img src="https://avatars.githubusercontent.com/u/61644087?v=4&amp;s=96" width="64" height="64" alt="@lpc520" title="@lpc520"></a>
  <a href="https://github.com/lijianchunChina"><img src="https://avatars.githubusercontent.com/u/42370856?v=4&amp;s=96" width="64" height="64" alt="@lijianchunChina" title="@lijianchunChina"></a>
  <a href="https://github.com/kjiojio"><img src="https://avatars.githubusercontent.com/u/118580250?v=4&amp;s=96" width="64" height="64" alt="@kjiojio" title="@kjiojio"></a>
  <a href="https://github.com/dmh-git-cop"><img src="https://avatars.githubusercontent.com/u/270353192?v=4&amp;s=96" width="64" height="64" alt="@dmh-git-cop" title="@dmh-git-cop"></a>
  <a href="https://github.com/ZZRSCAR"><img src="https://avatars.githubusercontent.com/u/255126066?v=4&amp;s=96" width="64" height="64" alt="@ZZRSCAR" title="@ZZRSCAR"></a>
  <a href="https://github.com/Toomi0124"><img src="https://avatars.githubusercontent.com/u/300393761?v=4&amp;s=96" width="64" height="64" alt="@Toomi0124" title="@Toomi0124"></a>
  <a href="https://github.com/Lezhao0226"><img src="https://avatars.githubusercontent.com/u/72743280?v=4&amp;s=96" width="64" height="64" alt="@Lezhao0226" title="@Lezhao0226"></a>
  <a href="https://github.com/HSsnano"><img src="https://avatars.githubusercontent.com/u/87816341?v=4&amp;s=96" width="64" height="64" alt="@HSsnano" title="@HSsnano"></a>
  <a href="https://github.com/zwbao"><img src="https://avatars.githubusercontent.com/u/24564677?v=4&amp;s=96" width="64" height="64" alt="@zwbao" title="@zwbao"></a>
  <a href="https://github.com/yuzhenpeng"><img src="https://avatars.githubusercontent.com/u/31943277?v=4&amp;s=96" width="64" height="64" alt="@yuzhenpeng" title="@yuzhenpeng"></a>
  <a href="https://github.com/youxiudongdong-lang"><img src="https://avatars.githubusercontent.com/u/306058340?v=4&amp;s=96" width="64" height="64" alt="@youxiudongdong-lang" title="@youxiudongdong-lang"></a>
  <a href="https://github.com/ying-ge"><img src="https://avatars.githubusercontent.com/u/45988974?v=4&amp;s=96" width="64" height="64" alt="@ying-ge" title="@ying-ge"></a>
  <a href="https://github.com/yemiaoyong"><img src="https://avatars.githubusercontent.com/u/61010663?v=4&amp;s=96" width="64" height="64" alt="@yemiaoyong" title="@yemiaoyong"></a>
  <a href="https://github.com/yejia1988"><img src="https://avatars.githubusercontent.com/u/164177661?v=4&amp;s=96" width="64" height="64" alt="@yejia1988" title="@yejia1988"></a>
  <a href="https://github.com/xingzhuo123"><img src="https://avatars.githubusercontent.com/u/167210517?v=4&amp;s=96" width="64" height="64" alt="@xingzhuo123" title="@xingzhuo123"></a>
  <a href="https://github.com/xiaochuheying19901216"><img src="https://avatars.githubusercontent.com/u/304343377?v=4&amp;s=96" width="64" height="64" alt="@xiaochuheying19901216" title="@xiaochuheying19901216"></a>
  <a href="https://github.com/xiahouzuoying"><img src="https://avatars.githubusercontent.com/u/57342415?v=4&amp;s=96" width="64" height="64" alt="@xiahouzuoying" title="@xiahouzuoying"></a>
  <a href="https://github.com/likemoonriver"><img src="https://avatars.githubusercontent.com/u/157043962?v=4&amp;s=96" width="64" height="64" alt="@likemoonriver" title="@likemoonriver"></a>
  <a href="https://github.com/lijianguoa"><img src="https://avatars.githubusercontent.com/u/52228119?v=4&amp;s=96" width="64" height="64" alt="@lijianguoa" title="@lijianguoa"></a>
  <a href="https://github.com/k1600639239"><img src="https://avatars.githubusercontent.com/u/301947158?v=4&amp;s=96" width="64" height="64" alt="@k1600639239" title="@k1600639239"></a>
  <a href="https://github.com/gongmeiyuan"><img src="https://avatars.githubusercontent.com/u/75189860?v=4&amp;s=96" width="64" height="64" alt="@gongmeiyuan" title="@gongmeiyuan"></a>
  <a href="https://github.com/chhhhai"><img src="https://avatars.githubusercontent.com/u/99796066?v=4&amp;s=96" width="64" height="64" alt="@chhhhai" title="@chhhhai"></a>
  <a href="https://github.com/chenchen199401-cmyk"><img src="https://avatars.githubusercontent.com/u/236738705?v=4&amp;s=96" width="64" height="64" alt="@chenchen199401-cmyk" title="@chenchen199401-cmyk"></a>
  <a href="https://github.com/btzheng"><img src="https://avatars.githubusercontent.com/u/15546828?v=4&amp;s=96" width="64" height="64" alt="@btzheng" title="@btzheng"></a>
  <a href="https://github.com/Winteric123"><img src="https://avatars.githubusercontent.com/u/122366825?v=4&amp;s=96" width="64" height="64" alt="@Winteric123" title="@Winteric123"></a>
  <a href="https://github.com/ShixiangWang"><img src="https://avatars.githubusercontent.com/u/25057508?v=4&amp;s=96" width="64" height="64" alt="@ShixiangWang" title="@ShixiangWang"></a>
  <a href="https://github.com/ScholarlyLuck"><img src="https://avatars.githubusercontent.com/u/267531500?v=4&amp;s=96" width="64" height="64" alt="@ScholarlyLuck" title="@ScholarlyLuck"></a>
  <a href="https://github.com/Junweichengang"><img src="https://avatars.githubusercontent.com/u/41681007?v=4&amp;s=96" width="64" height="64" alt="@Junweichengang" title="@Junweichengang"></a>
  <a href="https://github.com/JarningGau"><img src="https://avatars.githubusercontent.com/u/22016330?v=4&amp;s=96" width="64" height="64" alt="@JarningGau" title="@JarningGau"></a>
  <a href="https://github.com/Cloudy-Zhuang"><img src="https://avatars.githubusercontent.com/u/85553170?v=4&amp;s=96" width="64" height="64" alt="@Cloudy-Zhuang" title="@Cloudy-Zhuang"></a>
  <a href="https://github.com/245429488zc-svg"><img src="https://avatars.githubusercontent.com/u/250579619?v=4&amp;s=96" width="64" height="64" alt="@245429488zc-svg" title="@245429488zc-svg"></a>
  <a href="https://github.com/chewice"><img src="https://avatars.githubusercontent.com/u/244145152?v=4&amp;s=96" width="64" height="64" alt="@chewice" title="@chewice"></a>
  <a href="https://github.com/XuuChen"><img src="https://avatars.githubusercontent.com/u/99383234?v=4&amp;s=96" width="64" height="64" alt="@XuuChen" title="@XuuChen"></a>
</p>

Windows 代码签名由 [SignPath.io](https://signpath.io) 提供，证书由
[SignPath Foundation](https://signpath.org) 签发。第三方声明见
[开发指南](docs/development.md)。

## 许可证

除另有说明外，采用 [AGPL-3.0-only](LICENSE)。更早发布的版本继续适用其发布时
附带的许可证。

## 引用

[![DOI](https://zenodo.org/badge/1285857639.svg)](https://doi.org/10.5281/zenodo.21869520)

```bibtex
@software{xu2026wisp,
  author    = {Xu, Zhou-Geng},
  title     = {天成科研助手 Science: a local-first AI research workbench},
  version   = {v1.2.0},
  year      = {2026},
  publisher = {Zenodo},
  doi       = {10.5281/zenodo.21869520},
  url       = {https://doi.org/10.5281/zenodo.21869520}
}
```
