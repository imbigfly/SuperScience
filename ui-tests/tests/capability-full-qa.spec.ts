import { test, expect, type Page } from "@playwright/test";
import { writeFileSync } from "node:fs";
import { tauriMock } from "./mock-tauri";

type ActionKind =
  | "guided"
  | "install"
  | "toggle"
  | "none"
  | "settings"
  | "panel"
  | "demo"
  | "newchat"
  | "runtime";

type Cap = {
  id: string;
  group: string;
  title: string;
  kind: ActionKind;
  skill?: string;
  specialist?: string;
  turns: [string, string, string];
};

const CATALOG: Cap[] = [
  { id: "academic-paper", group: "论文写作与发表", title: "学术论文写作", kind: "guided", skill: "academic-paper", turns: ["主题：PD-1 耐药机制综述，目标期刊 Nature Reviews Immunology，模式 full。", "先出 1 页大纲，不要再追问。", "把引言第一段按大纲写出来。"] },
  { id: "academic-pipeline", group: "论文写作与发表", title: "研究到成稿流水线", kind: "guided", skill: "academic-pipeline", turns: ["从文献调研起步，交付物是可投稿初稿。", "先给阶段清单和第一阶段动作。", "只做调研阶段的检索计划。"] },
  { id: "academic-paper-reviewer", group: "论文写作与发表", title: "国际期刊审稿", kind: "guided", skill: "academic-paper-reviewer", turns: ["审一篇虚构的单细胞论文，期刊 Nature Methods。", "先列审稿角色和每人关注点。", "用魔鬼代言人写 3 条最狠意见。"] },
  { id: "deep-research", group: "选题与立项", title: "深度学术调研", kind: "guided", skill: "deep-research", turns: ["调研 CAR-T 在实体瘤的 2023 后进展。", "先给检索式和纳入标准。", "列出 5 个必须核验的主张。"] },
  { id: "nature-writing", group: "论文写作与发表", title: "Nature 风格起草", kind: "guided", skill: "nature-writing", turns: ["起草 Discussion，领域肿瘤免疫。", "先给段落骨架。", "写 120 字开篇段。"] },
  { id: "nature-polishing", group: "论文写作与发表", title: "Nature 风格润色", kind: "guided", skill: "nature-polishing", turns: ["润色这段：This result is very important and shows our method is better.", "保持原意，去掉空话。", "再给一个更克制的版本。"] },
  { id: "humanizer-zh", group: "论文写作与发表", title: "去除AI痕迹", kind: "guided", skill: "humanizer-zh", turns: ["改写：综上所述，本研究具有重要的理论意义与实践价值。", "指出命中了哪几类 AI 痕迹。", "再给一版更口语、仍学术的句子。"] },
  { id: "nature-proposal-writer", group: "选题与立项", title: "基金/开题写作", kind: "guided", skill: "nature-proposal-writer", turns: ["国自然面上，题目：肿瘤微环境中的髓系抑制。", "先写立项依据 5 条提纲。", "补一条创新点表述。"] },
  { id: "nature-citation", group: "论文写作与发表", title: "Nature 引用补全", kind: "guided", skill: "nature-citation", turns: ["给这段补引用：PD-1 blockade can restore T cell function.", "先说明你要查哪些库。", "用占位 DOI 写出一条 Nature 格式引用。"] },
  { id: "nature-response", group: "论文写作与发表", title: "修回信/答复信", kind: "guided", skill: "nature-response", turns: ["审稿人说样本量不够。请起草答复。", "先给答复策略（同意/部分同意/反驳）。", "写出 8 句正式答复。"] },
  { id: "nature-reviewer", group: "论文写作与发表", title: "Nature审稿", kind: "guided", skill: "nature-reviewer", turns: ["模拟 Nature 审稿，论文关于空间转录组。", "先给总体评价和推荐决定。", "写 3 条 Major comments。"] },
  { id: "nature-data", group: "论文写作与发表", title: "数据可用性 / FAIR", kind: "guided", skill: "nature-data", turns: ["RNA-seq 要上传 GEO，写 Data Availability。", "先列必须写清的字段。", "给一段可粘贴的 DAS 英文。"] },
  { id: "nature-ref-verifier", group: "论文写作与发表", title: "参考文献核验", kind: "guided", skill: "nature-ref-verifier", turns: ["核验：Smith et al. Nature 2020; 583:123-130.", "先说明核验步骤。", "如果查不到，应如何标记。"] },
  { id: "nature-paper-to-patent", group: "论文写作与发表", title: "论文转专利初稿", kind: "guided", skill: "nature-paper-to-patent", turns: ["把一篇方法学论文转成发明专利权利要求。", "先列独立权利要求要点。", "写权利要求 1 草案。"] },
  { id: "officecli", group: "论文写作与发表", title: "Word / Excel / PPT", kind: "guided", skill: "officecli", turns: ["用 OfficeCLI 建一个三页组会 PPT 大纲。", "先说明调用哪个技能命令。", "列出三页标题。"] },
  { id: "nature-figure", group: "AI画图", title: "投稿级科研作图", kind: "guided", skill: "nature-figure", turns: ["画一张 PD-1 通路机制示意图。", "先给图注和面板结构。", "用文字描述 Figure 1A。"] },
  { id: "r-bioinfo-figure", group: "AI画图", title: "R语言生信图", kind: "guided", skill: "nature-figure", specialist: "r_bioinformatics_figure", turns: ["用 R 画火山图，数据是 DEG 表。", "先给 ggplot 代码骨架。", "补上 padj 与 log2FC 阈值。"] },
  { id: "nature-paper2ppt", group: "AI画图", title: "论文转组会 PPT", kind: "guided", skill: "nature-paper2ppt", turns: ["把一篇方法论文转成 8 页组会 PPT。", "先给 8 页目录。", "写出第 2 页要点。"] },
  { id: "ppt-master", group: "AI画图", title: "PPT Master", kind: "install", skill: "ppt-master", turns: ["做一份 6 页可编辑 PPT，主题单细胞注释。", "先确认是否已安装技能。", "给出 6 页标题。"] },
  { id: "ai-drawing", group: "AI画图", title: "AI 插图生成", kind: "guided", turns: ["生成一张肿瘤免疫微环境概念图。", "先问模型与比例，但不要超过一个问题。", "给出英文生图提示词。"] },
  { id: "ai-mechanism-figure", group: "AI画图", title: "AI机制图", kind: "none", turns: ["点击入口", "确认未误开对话", "确认仍停留在能力页"] },
  { id: "editable-figure", group: "AI画图", title: "图片可编辑", kind: "none", turns: ["点击入口", "确认未误开对话", "确认仍停留在能力页"] },
  { id: "bioinfo-figure-layout", group: "AI画图", title: "生信图布局", kind: "none", turns: ["点击入口", "确认未误开对话", "确认仍停留在能力页"] },
  { id: "data-cleaning", group: "数据清洗", title: "数据清洗与整形", kind: "guided", specialist: "data_cleaning", turns: ["清洗一份临床随访 CSV，路径 data/followup.csv。", "先说明你会检查哪些问题。", "给出清洗后的列清单。"] },
  { id: "pii-firewall", group: "效率工具", title: "出站隐私脱敏", kind: "toggle", turns: ["查看开关与说明", "切换一次开关", "再切回并确认卡片仍在"] },
  { id: "journal-prescreen", group: "效率工具", title: "论文预审", kind: "guided", skill: "journal-prescreen", turns: ["预审一篇临床论著，目标《中华内科杂志》。", "先列你会对照的须知条目。", "标出 2 个格式问题并给改法。"] },
  { id: "handwriting-extract", group: "效率工具", title: "手写数据提取", kind: "guided", skill: "handwriting-extract", turns: ["识别两张手写 CRF 照片成 CSV。", "先说明如何标存疑格子。", "给出输出路径约定。"] },
  { id: "topic-coach", group: "效率工具", title: "选题引导", kind: "guided", skill: "topic-coach", turns: ["我有一份回顾性队列，想投中华系列。", "先列资料盘点要问什么。", "给 3 个选题候选的评分维度。"] },
  { id: "env-setup", group: "数据清洗", title: "配置本机环境", kind: "runtime", turns: ["打开本机环境准备面板", "确认未新开聊天", "收起后面板芯片仍在"] },
  { id: "nature-statistics", group: "研究实施与数据分析", title: "统计报告审核", kind: "guided", skill: "nature-statistics", turns: ["两组小鼠体重做 t 检验，n=8。", "先判断前提假设。", "给出 R 代码提纲。"] },
  { id: "knowledge-graph", group: "选题与立项", title: "文本知识图谱", kind: "guided", skill: "knowledge-graph", turns: ["从这段抽图谱：ESR1 抑制后 TNF 通路上调。", "先列实体类型。", "给出 3 条三元组。"] },
  { id: "stats-analysis", group: "研究实施与数据分析", title: "统计建模与检验", kind: "guided", turns: ["生存分析，终点 OS，协变量年龄和分期。", "先选模型。", "写出公式。"] },
  { id: "python-r", group: "研究实施与数据分析", title: "Python / R 分析", kind: "guided", turns: ["用 Python 读 counts.tsv 做 PCA。", "先给代码步骤。", "说明如何解释 PC1。"] },
  { id: "bio-db", group: "选题与立项", title: "生物数据库", kind: "guided", turns: ["查 TP53 在 UniProt 的功能摘要。", "先说明用哪个 MCP/库。", "列出你会返回的字段。"] },
  { id: "remote-compute", group: "研究实施与数据分析", title: "远程算力与长任务", kind: "settings", turns: ["打开环境设置", "确认能看到环境分区", "关闭设置回到能力页"] },
  { id: "structure", group: "研究实施与数据分析", title: "结构预测与分子设计", kind: "guided", turns: ["预测一条 80aa 肽的结构。", "先说明工具边界。", "给出下一步输入要求。"] },
  { id: "single-cell", group: "研究实施与数据分析", title: "单细胞与分析流程", kind: "guided", turns: ["10x 单细胞注释，物种人。", "先给标准流程。", "说明如何交文件。"] },
  { id: "academic-search-pro", group: "选题与立项", title: "跨库文献检索", kind: "guided", skill: "academic-search-pro", turns: ["检索 ferroptosis AND immunotherapy，2022 后。", "先说会查哪些库。", "说明去重和 BibTeX 交付。"] },
  { id: "nature-academic-search", group: "选题与立项", title: "多源文献检索", kind: "guided", skill: "nature-academic-search", turns: ["PubMed 检索 NLRP3 与肝癌。", "先给 MeSH/关键词。", "说明是否做他引审计。"] },
  { id: "nature-literature-pipeline", group: "选题与立项", title: "文献流水线", kind: "guided", skill: "nature-literature-pipeline", turns: ["做一套纳入/排除标准后的文献流水线。", "先问领域，我已给：免疫治疗耐药。", "列出 5 个阶段。"] },
  { id: "nature-downloader", group: "选题与立项", title: "合法全文获取", kind: "guided", skill: "nature-downloader", turns: ["用 DOI 10.1038/s41586-020-0001-0 走 OA 路径。", "先说明不会绕过付费墙。", "给出保存位置建议。"] },
  { id: "nature-paper-card", group: "选题与立项", title: "深读论文卡片", kind: "guided", skill: "nature-paper-card", turns: ["为一篇虚构 Nature 论文做 Paper Card。", "先列卡片栏目。", "写主张与局限各一条。"] },
  { id: "nature-reader", group: "选题与立项", title: "中英对照阅读器", kind: "guided", skill: "nature-reader", turns: ["精读 Methods，关注统计是否充分。", "先给阅读清单。", "列出 3 个要核对的数字。"] },
  { id: "nature-experiment-log", group: "研究实施与数据分析", title: "实验日志", kind: "guided", skill: "nature-experiment-log", turns: ["记录今天的 Western blot，抗体 PD-1。", "先给记录模板。", "补上对照和批次字段。"] },
  { id: "research-graph", group: "研究设计与规划", title: "研究图谱", kind: "panel", turns: ["打开研究图谱面板", "确认面板可见", "关闭后回到能力页"] },
  { id: "publication", group: "论文写作与发表", title: "论文证据工作区", kind: "panel", turns: ["打开发表面板", "确认面板可见", "关闭后回到能力页"] },
  { id: "files-library", group: "研究设计与规划", title: "文件与收藏库", kind: "panel", turns: ["打开文件面板", "确认面板可见", "关闭后回到能力页"] },
  { id: "demo", group: "研究设计与规划", title: "打开演示", kind: "demo", turns: ["打开演示项目", "确认只读演示可见", "返回项目页"] },
  { id: "channels", group: "协作扩展", title: "飞书 / 微信接入", kind: "settings", turns: ["打开频道设置", "确认设置分区可见", "关闭设置"] },
  { id: "browser", group: "协作扩展", title: "真实浏览器", kind: "guided", turns: ["列出当前可做的浏览器自动化边界。", "不要尝试绕过验证码。", "给一个只读标签页的示例请求。"] },
  { id: "playwright", group: "协作扩展", title: "Playwright 无头浏览器", kind: "guided", skill: "playwright", turns: ["写一个打开 example.com 并取标题的脚本提纲。", "先说明本机如何安装运行时。", "给出 5 行伪代码。"] },
  { id: "plugins", group: "协作扩展", title: "插件扩展", kind: "settings", turns: ["打开插件设置", "确认设置分区可见", "关闭设置"] },
];

test.use({ locale: "zh-CN" });
test.setTimeout(20 * 60_000);

test.beforeEach(async ({ page }) => {
  await page.addInitScript(tauriMock);
});

async function lastInvoke(page: Page, cmd: string) {
  return page.evaluate((name) => {
    const plain = (value: any): any => {
      if (value instanceof Map) return Object.fromEntries([...value].map(([k, v]) => [k, plain(v)]));
      if (Array.isArray(value)) return value.map(plain);
      if (value && typeof value === "object") {
        return Object.fromEntries(Object.entries(value).map(([k, v]) => [k, plain(v)]));
      }
      return value;
    };
    const calls = ((window as any).__skillInvokeLog ?? []).filter((c: any) => c.cmd === name);
    return plain(calls.at(-1)?.args ?? null);
  }, cmd);
}

async function invokeCount(page: Page, cmd: string) {
  return page.evaluate((name) => {
    return ((window as any).__skillInvokeLog ?? []).filter((c: any) => c.cmd === name).length;
  }, cmd);
}

async function goHome(page: Page) {
  const installCancel = page.getByTestId("catalog-skill-install-cancel");
  if (await installCancel.isVisible().catch(() => false)) {
    await installCancel.click({ timeout: 5_000 });
  }
  const overlay = page.locator(".overlay").last();
  if (await overlay.isVisible().catch(() => false)) {
    await overlay.click({ position: { x: 8, y: 8 }, timeout: 5_000, force: true }).catch(() => {});
  }
  for (let i = 0; i < 4; i += 1) {
    await page.keyboard.press("Escape").catch(() => {});
  }
  const back = page.getByRole("button", { name: "返回项目" });
  if (await back.isVisible().catch(() => false)) {
    await back.click({ timeout: 5_000, force: true }).catch(() => {});
  }
  await expect(page.getByTestId("capability-scene")).toBeVisible({ timeout: 15_000 });
}

async function openGroup(page: Page, group: string) {
  await page.getByRole("tab", { name: group }).click({ timeout: 8_000 });
  await expect(page.getByRole("tab", { name: group })).toHaveAttribute("aria-selected", "true");
  await expect(page.getByTestId("capability-tile-grid")).toBeVisible();
}

async function sendFollowUp(page: Page, text: string) {
  const box = page.locator("#composer-input");
  await expect(box).toBeVisible({ timeout: 10_000 });
  await box.fill(text);
  await box.press("Enter");
}

async function waitAssistant(page: Page, minCount: number) {
  await expect.poll(async () => page.locator(".msg.assistant").count(), { timeout: 12_000 })
    .toBeGreaterThanOrEqual(minCount);
}

function scoreResult(kind: ActionKind, rec: Record<string, unknown>): { interact: number; reply: number; notes: string[] } {
  const notes: string[] = [];
  let interact = 0;
  let reply = 0;
  if (rec.tileVisible) interact += 20; else notes.push("卡片不可见");
  if (rec.clickOk) interact += 20; else notes.push("点击失败");
  if (kind === "install" && rec.surfaceOk && !rec.sessionOpened) {
    interact += 60;
    reply = 70;
    notes.push("首次点击正确弹出安装确认，未预装属预期");
  } else if (kind === "guided" || kind === "newchat" || kind === "install") {
    if (rec.sessionOpened) interact += 20; else notes.push("未进入会话");
    if (rec.skillOk !== false) interact += 15; else notes.push("技能引用不匹配");
    if (rec.titleOk !== false) interact += 10; else notes.push("会话标题不匹配");
    if (Number(rec.rounds) >= 3) interact += 15; else notes.push(`对话轮次不足：${rec.rounds}`);
    const replies = Number(rec.assistantReplies ?? 0);
    reply = Math.min(100, replies * 30 + (replies >= 3 ? 10 : 0));
    if (replies < 3) notes.push("助手回复不足 3 条");
    notes.push("助手正文来自 UI 测试桥接，不是线上大模型");
  } else if (kind === "none") {
    if (rec.stayedOnHome) interact += 60; else notes.push("占位卡误触发了其它界面");
    reply = rec.stayedOnHome ? 70 : 20;
    notes.push("占位卡：产品尚未接线对话");
  } else if (kind === "toggle") {
    if (rec.toggleOk) interact += 60; else notes.push("开关未响应");
    reply = rec.toggleOk ? 80 : 30;
  } else if (kind === "settings" || kind === "panel" || kind === "demo" || kind === "runtime") {
    if (rec.surfaceOk) interact += 60; else notes.push("未打开预期界面");
    reply = rec.surfaceOk ? 75 : 25;
  }
  return { interact: Math.min(100, interact), reply: Math.min(100, reply), notes };
}

test("full capability click-through with three dialogue rounds", async ({ page }) => {
  test.skip(!process.env.QA_FULL, "set QA_FULL=1 to run the full capability QA sweep");
  await page.goto("/?mockLocale=zh");
  await expect(page.getByTestId("capability-scene")).toBeVisible();

  const results: any[] = [];

  for (const cap of CATALOG) {
    const rec: any = {
      id: cap.id,
      group: cap.group,
      title: cap.title,
      kind: cap.kind,
      skill: cap.skill ?? null,
      specialist: cap.specialist ?? null,
      turns: [] as { role: string; text: string }[],
      tileVisible: false,
      clickOk: false,
      sessionOpened: false,
      skillOk: true,
      titleOk: true,
      stayedOnHome: false,
      toggleOk: false,
      surfaceOk: false,
      rounds: 0,
      assistantReplies: 0,
      firstPrompt: "",
      error: "",
    };

    try {
      await goHome(page);
      await openGroup(page, cap.group);
      const tile = page.getByTestId(`cap-tile-${cap.id}`);
      rec.tileVisible = await tile.isVisible();
      if (!rec.tileVisible) {
        rec.error = "能力卡不可见";
        const scored = scoreResult(cap.kind, rec);
        results.push({ ...rec, ...scored, total: Math.round(scored.interact * 0.6 + scored.reply * 0.4) });
        continue;
      }

      const sendsBefore = await invokeCount(page, "send_message");
      if (cap.kind !== "toggle") {
        try {
          await tile.click({ timeout: 8_000 });
          rec.clickOk = true;
        } catch (error) {
          const msg = error instanceof Error ? error.message : String(error);
          if (/not attached|detached from the DOM/i.test(msg)) {
            rec.clickOk = true;
          } else {
            throw error;
          }
        }
      }

      if (cap.kind === "none") {
        rec.stayedOnHome = await page.getByTestId("capability-scene").isVisible();
        rec.rounds = 3;
        rec.turns = cap.turns.map((text) => ({ role: "检查", text }));
      } else if (cap.kind === "toggle") {
        const sw = page.getByTestId("cap-tile-pii-firewall-switch");
        rec.toggleOk = await sw.isVisible();
        rec.clickOk = rec.toggleOk;
        if (rec.toggleOk) {
          const before = await invokeCount(page, "set_pii_firewall_enabled");
          await sw.click();
          await expect.poll(() => invokeCount(page, "set_pii_firewall_enabled")).toBeGreaterThan(before);
          await sw.click();
          rec.toggleOk = (await invokeCount(page, "set_pii_firewall_enabled")) >= before + 2;
        }
        rec.stayedOnHome = await page.getByTestId("capability-scene").isVisible();
        rec.rounds = 3;
        rec.turns = cap.turns.map((text) => ({ role: "操作", text }));
      } else if (cap.kind === "settings") {
        const settingsVisible = async () =>
          await page.locator(".settings-nav, .settings-page, [data-testid='settings-dialog']").first().isVisible().catch(() => false)
          || await page.getByRole("button", { name: /常规|远程接入|插件|环境/ }).first().isVisible().catch(() => false)
          || await page.getByRole("heading", { name: /环境|频道|插件|设置|远程接入/ }).first().isVisible().catch(() => false);
        rec.surfaceOk = await settingsVisible();
        if (!rec.surfaceOk) {
          await tile.click({ timeout: 8_000 }).catch(() => {});
          rec.surfaceOk = await settingsVisible();
        }
        rec.rounds = 3;
        rec.turns = cap.turns.map((text) => ({ role: "操作", text }));
        await page.keyboard.press("Escape");
      } else if (cap.kind === "panel") {
        rec.surfaceOk = !(await page.getByTestId("capability-scene").isVisible().catch(() => false))
          || await page.locator(".right-panel, .graph-panel, [data-testid='files-panel'], [data-testid='agents-panel']").first().isVisible().catch(() => false);
        rec.rounds = 3;
        rec.turns = cap.turns.map((text) => ({ role: "操作", text }));
        await page.keyboard.press("Escape");
      } else if (cap.kind === "runtime") {
        rec.surfaceOk = await page.getByTestId("runtime-setup-panel").isVisible().catch(() => false);
        rec.sessionOpened = (await page.locator(".msg.user").count()) > 0;
        rec.skillOk = !rec.sessionOpened;
        rec.rounds = 3;
        rec.turns = cap.turns.map((text) => ({ role: "操作", text }));
        await page.keyboard.press("Escape");
      } else if (cap.kind === "demo") {
        rec.surfaceOk = await page.getByTestId("demo-read-only").isVisible().catch(() => false);
        rec.rounds = 3;
        rec.turns = cap.turns.map((text) => ({ role: "操作", text }));
      } else if (cap.kind === "install") {
        const install = page.getByTestId("catalog-skill-install-dialog");
        const openedInstall = await install.isVisible().catch(() => false);
        if (openedInstall) {
          rec.surfaceOk = true;
          rec.rounds = 3;
          rec.turns = cap.turns.map((text) => ({ role: "操作", text }));
          const cancel = page.getByTestId("catalog-skill-install-cancel");
          if (await cancel.isVisible().catch(() => false)) {
            await cancel.click({ timeout: 5_000 });
          } else {
            await page.keyboard.press("Escape");
          }
        } else {
          rec.sessionOpened = await page.locator("#composer-input").isVisible().catch(() => false);
          const sent = await lastInvoke(page, "send_message");
          rec.firstPrompt = String(sent?.message ?? "");
          rec.skillOk = !cap.skill || JSON.stringify(sent?.references ?? []).includes(cap.skill);
          rec.turns.push({ role: "用户（产品自动首轮）", text: rec.firstPrompt.slice(0, 800) });
          await waitAssistant(page, 1);
          await sendFollowUp(page, cap.turns[1]);
          rec.turns.push({ role: "用户", text: cap.turns[1] });
          await waitAssistant(page, 2);
          await sendFollowUp(page, cap.turns[2]);
          rec.turns.push({ role: "用户", text: cap.turns[2] });
          await waitAssistant(page, 3);
          rec.rounds = 3;
          rec.assistantReplies = await page.locator(".msg.assistant").count();
          const replies = await page.locator(".msg.assistant").allInnerTexts();
          for (const text of replies.slice(0, 3)) {
            rec.turns.push({ role: "助手", text: text.slice(0, 500) });
          }
        }
      } else if (cap.kind === "newchat") {
        rec.sessionOpened = await page.locator("#composer-input").isVisible({ timeout: 10_000 }).catch(() => false);
        await sendFollowUp(page, cap.turns[0]);
        rec.turns.push({ role: "用户", text: cap.turns[0] });
        await waitAssistant(page, 1);
        await sendFollowUp(page, cap.turns[1]);
        rec.turns.push({ role: "用户", text: cap.turns[1] });
        await waitAssistant(page, 2);
        await sendFollowUp(page, cap.turns[2]);
        rec.turns.push({ role: "用户", text: cap.turns[2] });
        await waitAssistant(page, 3);
        rec.rounds = 3;
        rec.assistantReplies = await page.locator(".msg.assistant").count();
        const replies = await page.locator(".msg.assistant").allInnerTexts();
        for (const text of replies.slice(0, 3)) {
          rec.turns.push({ role: "助手", text: text.replace(/\s+/g, " ").slice(0, 400) });
        }
      } else {
        await expect.poll(async () => invokeCount(page, "send_message"), { timeout: 12_000 })
          .toBeGreaterThan(sendsBefore);
        rec.sessionOpened = await page.locator("#composer-input").isVisible();
        const sent = await lastInvoke(page, "send_message");
        rec.firstPrompt = String(sent?.message ?? "");
        if (cap.skill) {
          rec.skillOk = JSON.stringify(sent?.references ?? []).includes(cap.skill);
        }
        if (cap.specialist) {
          const spec = await lastInvoke(page, "set_session_specialist");
          rec.skillOk = rec.skillOk && String(spec?.id ?? "") === cap.specialist;
        }
        const renamed = await lastInvoke(page, "rename_session");
        rec.titleOk = !renamed || String(renamed.title ?? "").includes(cap.title);
        rec.turns.push({ role: "用户（产品自动首轮）", text: rec.firstPrompt.slice(0, 1200) });
        await waitAssistant(page, 1);
        await sendFollowUp(page, cap.turns[1]);
        rec.turns.push({ role: "用户", text: cap.turns[1] });
        await waitAssistant(page, 2);
        await sendFollowUp(page, cap.turns[2]);
        rec.turns.push({ role: "用户", text: cap.turns[2] });
        await waitAssistant(page, 3);
        rec.rounds = 3;
        rec.assistantReplies = await page.locator(".msg.assistant").count();
        const replies = await page.locator(".msg.assistant").allInnerTexts();
        for (const text of replies.slice(0, 3)) {
          rec.turns.push({ role: "助手", text: text.replace(/\s+/g, " ").slice(0, 400) });
        }
      }
    } catch (error) {
      rec.error = error instanceof Error ? error.message : String(error);
      rec.clickOk = rec.clickOk || false;
    }

    const scored = scoreResult(cap.kind, rec);
    results.push({
      ...rec,
      interact: scored.interact,
      reply: scored.reply,
      notes: scored.notes,
      total: Math.round(scored.interact * 0.6 + scored.reply * 0.4),
    });
    writeFileSync("/tmp/capability-qa-results.json", JSON.stringify({
      generatedAt: new Date().toISOString(),
      product: "天成科研助手",
      version: "1.5.0",
      method: "Playwright 真实点击产品 UI（中文 locale），三轮对话发送；助手回复来自 UI 测试桥接",
      results,
    }, null, 2));
  }

  const out = {
    generatedAt: new Date().toISOString(),
    product: "天成科研助手",
    version: "1.5.0",
    method: "Playwright 真实点击产品 UI（中文 locale），三轮对话发送；助手回复来自 UI 测试桥接",
    results,
  };
  writeFileSync("/tmp/capability-qa-results.json", JSON.stringify(out, null, 2));
  expect(results.length).toBe(CATALOG.length);
});
