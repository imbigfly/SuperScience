//! Full-screen capability intro copy. Kept out of `i18n.rs` and the tile catalog.

use crate::i18n::Locale;

#[derive(Debug, Clone, Copy)]
pub(crate) struct CapabilityHelp {
    en: &'static str,
    zh: &'static str,
}

impl CapabilityHelp {
    pub(crate) fn get(self, locale: Locale) -> &'static str {
        match locale {
            Locale::En => self.en,
            Locale::Zh => self.zh,
        }
    }
}

pub(crate) const fn help(en: &'static str, zh: &'static str) -> CapabilityHelp {
    CapabilityHelp { en, zh }
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct CapabilityIntro {
    pub scenario: CapabilityHelp,
    pub problems: CapabilityHelp,
    pub value: CapabilityHelp,
    pub how_to: CapabilityHelp,
    pub dialogue_user: CapabilityHelp,
    pub dialogue_assistant: CapabilityHelp,
    #[allow(dead_code)]
    pub example_project: Option<&'static str>,
    #[allow(dead_code)]
    pub tutorial_url: Option<&'static str>,
}

const fn intro(
    scenario: CapabilityHelp,
    problems: CapabilityHelp,
    value: CapabilityHelp,
    how_to: CapabilityHelp,
    dialogue_user: CapabilityHelp,
    dialogue_assistant: CapabilityHelp,
) -> CapabilityIntro {
    CapabilityIntro {
        scenario,
        problems,
        value,
        how_to,
        dialogue_user,
        dialogue_assistant,
        example_project: None,
        tutorial_url: None,
    }
}

pub(crate) fn intro_lines(text: &str) -> impl Iterator<Item = &str> {
    text.lines().map(str::trim).filter(|line| !line.is_empty())
}

pub(crate) fn intro_value_parts(line: &str) -> (&str, Option<&str>) {
    match line.split_once(" | ") {
        Some((title, body)) => (title.trim(), Some(body.trim())),
        None => (line, None),
    }
}

pub(crate) fn intro_for(id: &str) -> Option<&'static CapabilityIntro> {
    Some(match id {
        "deep-research" => &DEEP_RESEARCH,
        "nature-proposal-writer" => &NATURE_PROPOSAL_WRITER,
        "knowledge-graph" => &KNOWLEDGE_GRAPH,
        "bio-db" => &BIO_DB,
        "academic-search-pro" => &ACADEMIC_SEARCH_PRO,
        "nature-academic-search" => &NATURE_ACADEMIC_SEARCH,
        "nature-literature-pipeline" => &NATURE_LITERATURE_PIPELINE,
        "nature-downloader" => &NATURE_DOWNLOADER,
        "nature-paper-card" => &NATURE_PAPER_CARD,
        "nature-reader" => &NATURE_READER,
        "research-graph" => &RESEARCH_GRAPH,
        "files-library" => &FILES_LIBRARY,
        "demo" => &DEMO,
        "nature-statistics" => &NATURE_STATISTICS,
        "stats-analysis" => &STATS_ANALYSIS,
        "python-r" => &PYTHON_R,
        "remote-compute" => &REMOTE_COMPUTE,
        "structure" => &STRUCTURE,
        "single-cell" => &SINGLE_CELL,
        "nature-experiment-log" => &NATURE_EXPERIMENT_LOG,
        "academic-paper" => &ACADEMIC_PAPER,
        "academic-pipeline" => &ACADEMIC_PIPELINE,
        "academic-paper-reviewer" => &ACADEMIC_PAPER_REVIEWER,
        "nature-writing" => &NATURE_WRITING,
        "nature-polishing" => &NATURE_POLISHING,
        "humanizer-zh" => &HUMANIZER_ZH,
        "nature-citation" => &NATURE_CITATION,
        "nature-response" => &NATURE_RESPONSE,
        "nature-reviewer" => &NATURE_REVIEWER,
        "nature-data" => &NATURE_DATA,
        "nature-ref-verifier" => &NATURE_REF_VERIFIER,
        "nature-paper-to-patent" => &NATURE_PAPER_TO_PATENT,
        "officecli" => &OFFICECLI,
        "publication" => &PUBLICATION,
        "nature-figure" => &NATURE_FIGURE,
        "r-bioinfo-figure" => &R_BIOINFO_FIGURE,
        "ai-drawing" => &AI_DRAWING,
        "ai-mechanism-figure" => &AI_MECHANISM_FIGURE,
        "editable-figure" => &EDITABLE_FIGURE,
        "bioinfo-figure-layout" => &BIOINFO_FIGURE_LAYOUT,
        "nature-paper2ppt" => &NATURE_PAPER2PPT,
        "nature-image2ppt" => &NATURE_IMAGE2PPT,
        "ppt-master" => &PPT_MASTER,
        "data-cleaning" => &DATA_CLEANING,
        "env-setup" => &ENV_SETUP,
        "pii-firewall" => &PII_FIREWALL,
        "journal-prescreen" => &JOURNAL_PRESCREEN,
        "figure-duplicate-audit" => &FIGURE_DUPLICATE_AUDIT,
        "handwriting-extract" => &HANDWRITING_EXTRACT,
        "topic-coach" => &TOPIC_COACH,
        "knowledge" => &KNOWLEDGE,
        "channels" => &CHANNELS,
        "browser" => &BROWSER,
        "playwright" => &PLAYWRIGHT,
        "plugins" => &PLUGINS,
        "skills" => &SKILLS,
        _ => return None,
    })
}

const DEEP_RESEARCH: CapabilityIntro = intro(
    help(
        "Before a proposal or review, you need a cited brief on a research question, not a pile of search links.\nClick the card to start a guided chat, then say something as specific as: “Brief me on spatial transcriptomics in the tumor microenvironment since 2020. Experimental papers only, and give a cited Chinese brief”.",
        "开题前要把一个研究问题摸清；写综述或基金背景时需要带引用的简报，而不是搜索引擎链接堆。\n点卡片，打开一场带引导的对话，第一句尽量写具体，例如：「帮我调研 2020 年以来空间转录组在肿瘤微环境中的应用，只要有实验证据的工作，出一份带引用的中文简报」。",
    ),
    help(
        "Papers that answer the same question sit in different tabs, so you cannot see the field as one argument.\nFacts look fine in a skim and fall apart when you try to cite them in a proposal.\nCitations get invented or pasted after the conclusion is already written.\nYou cannot tell which few papers actually matter, or where the real debate is.",
        "同一问题的文献散在各个标签页和文件夹里，看不到整张图景。\n略读时觉得站得住的事实，写进基金或综述一对出处就垮。\n结论先写完，引用事后补，甚至会编出不存在的条目。\n分不清该精读哪几篇，争议点在哪，下一手也接不上。",
    ),
    help(
        "A cited brief with sources you can reopen | Each claim points back to a paper you can open, grouped by mechanism, method, and translation.\nConsensus, debate, and evidence gaps marked separately | The brief does not flatten the field: it says where authors agree, where they fight, and what is still missing.\nFewer “decide first, cite later” drafts | You write from the brief instead of inventing a story and hunting citations afterwards.\nReusable entries for later writing | The same entries can drop into a proposal, review, or related-work section later.",
        "一次给出可回溯出处的调研简报 | 每条主张都能点回可打开的文献，并按机制、方法、转化分节。\n标出共识、争议和证据缺口 | 不会把领域抹平：谁同意、谁在争、还缺什么，分开写清。\n减少「先下结论再补文献」 | 先有简报再动笔，而不是写完故事再找引用。\n后续写作能直接引用条目 | 同一批条目可以进基金、综述或相关工作。",
    ),
    help(
        "Click the card to start a guided chat\nState the question, field, and year window in one sentence\nAdd constraints such as experimental evidence only or a Chinese brief\nStart with the actual task, for example: “Brief me on spatial transcriptomics in the tumor microenvironment since 2020. Experimental papers only, and give a cited Chinese brief.”",
        "点卡片，打开一场带引导的对话\n用一句话写清研究问题、领域和年限\n需要的话补上只要实验证据、只要中文简报等约束\n第一句直接进入任务，可参考：「帮我调研 2020 年以来空间转录组在肿瘤微环境中的应用，只要有实验证据的工作，出一份带引用的中文简报。」",
    ),
    help(
        "Brief me on spatial transcriptomics in the tumor microenvironment since 2020. Experimental papers only, and give a cited Chinese brief.",
        "帮我调研 2020 年以来空间转录组在肿瘤微环境中的应用，只要有实验证据的工作，出一份带引用的中文简报。",
    ),
    help(
        "I will freeze the scope and inclusion rules, then search and check claims, and return a cited brief split by mechanism, method, and translation.",
        "先限定综述范围与纳入标准，再检索并核对主张，按机制、技术和临床转化分节给出带出处的简报。",
    ),
);

const NATURE_PROPOSAL_WRITER: CapabilityIntro = intro(
    help(
        "Grants, opening reports, and research plans that must survive committee review.\nClick the card and paste the title, call text, and notes, then say something as specific as: “Here is an NSFC general-program excerpt and my three-page outline. Tighten the scientific question and technical route. No empty novelty claims”.",
        "国自然、重点研发、校内开题、中期或研究计划要写成可评审的本子。\n点卡片，贴题目、指南摘要和已有提纲，第一句尽量写具体，例如：「这是国自然面上指南摘录和我的三页提纲，帮我改科学问题和技术路线，创新点不要空话」。",
    ),
    help(
        "The scientific question stays vague, so reviewers treat novelty as a slogan.\nAims, technical route, and milestones do not line up on the same page.\nEach funder template forces a full reformat of the same idea.\nRisk plans, controls, and fallback routes are missing until the committee asks.",
        "科学问题虚，创新点读起来像口号，评委抓不住要评什么。\n目标、技术路线和考核指标对不齐，本子内部自相矛盾。\n换一套基金模板就要把同一想法整份重排。\n对照、风险和备选路线要到会上才被追问。",
    ),
    help(
        "Turns an idea into question, hypothesis, route, and milestones | The outline becomes a testable question, a route a reviewer can follow, and dates you can be held to.\nFlags the weak paragraphs a reviewer will attack | Empty novelty, missing controls, and unaligned aims are marked before the committee sees them.\nRevises an existing outline instead of starting over | You keep the pages you already have and rewrite the weak joints.\nCall-text keywords stay visible | The draft keeps the funder’s language without turning into keyword stuffing.",
        "把想法落成「问题—假设—路线—指标」 | 提纲变成可检验的问题、评委跟得上的路线，以及能被考核的节点。\n标出评审会追问的弱段 | 空创新、缺对照、目标对不齐的段落，会前先标出来。\n可按已有提纲改，而不必重写 | 已有页保留，只改接不上的关节。\n指南关键词仍看得见 | 本子对齐指南用语，但不会堆成关键词墙。",
    ),
    help(
        "Click the card and paste the title, call text, and notes\nName the fund type and the page or word limit\nSay whether you want a full draft or a critique\nStart with the actual task, for example: “Here is an NSFC general-program excerpt and my three-page outline. Tighten the scientific question and technical route. No empty novelty claims.”",
        "点卡片，贴题目、指南摘要和已有提纲\n说明资助类型与篇幅\n说清要全文起草还是只改弱段\n第一句直接进入任务，可参考：「这是国自然面上指南摘录和我的三页提纲，帮我改科学问题和技术路线，创新点不要空话。」",
    ),
    help(
        "Here is an NSFC general-program excerpt and my three-page outline. Tighten the scientific question and technical route. No empty novelty claims.",
        "这是国自然面上指南摘录和我的三页提纲，帮我改科学问题和技术路线，创新点不要空话。",
    ),
    help(
        "I will align the call keywords, rewrite a testable hypothesis, and mark missing controls and risk plans in the route.",
        "先对齐指南关键词，再重写科学问题与可检验假设，并标出路线里缺的对照和风险对策。",
    ),
);

const KNOWLEDGE_GRAPH: CapabilityIntro = intro(
    help(
        "After reading several papers, you need a searchable, draggable map of who does what to whom.\nClick the card and give a PDF, notes, or pasted paragraphs, then say something as specific as: “Turn this immunotherapy review into a knowledge graph. Focus on pathways, ligand-receptors, and trials”.",
        "精读多篇后要看实体和关系；组会要一张可拖动的关系图。\n点卡片，给论文 PDF、笔记或粘贴段落，第一句尽量写具体，例如：「把这篇免疫治疗综述做成知识图谱，重点抽通路、配体受体、临床试验」。",
    ),
    help(
        "Who-does-what notes get lost inside PDFs and cannot be searched later.\nMechanism chains live only in your head, so a collaborator cannot reuse them.\nA slide list of entities cannot answer “what binds what”.\nThe same ligand or trial appears under three names and never gets merged.",
        "「谁对谁做了什么」埋在 PDF 和笔记里，过后搜不回来。\n机制链条只在脑子里，协作的人无法复用。\n幻灯片清单回答不了「谁和谁结合」。\n同一配体或试验有三个名字，从未合并。",
    ),
    help(
        "Extracted triples visualized as a graph | Entities and relations become a draggable map you can search, not a bullet list.\nSearch by entity | Type a pathway, drug, or trial and see every edge that touches it.\nA map you can reuse in writing or a later brief | The same graph feeds a related-work paragraph or a later survey.\nDuplicates collapsed | The same molecule or trial is merged instead of appearing three times.",
        "抽出三元组并可视化 | 实体和关系变成可拖、可搜的图，而不是一条条清单。\n可按实体搜索 | 输入通路、药物或试验，就能看到连着它的边。\n图能留给后续写作或调研 | 同一张图能进相关工作或下一轮综述。\n同名实体会合并 | 同一分子或试验不会以三个名字各出现一次。",
    ),
    help(
        "Click the card and give a PDF, notes, or pasted paragraphs\nName the entity types to extract, such as pathways or trials\nStart with the actual task, for example: “Turn this immunotherapy review into a knowledge graph. Focus on pathways, ligand-receptors, and trials.”",
        "点卡片，给论文 PDF、笔记或粘贴段落\n说明要抽的实体类型，例如通路或临床试验\n第一句直接进入任务，可参考：「把这篇免疫治疗综述做成知识图谱，重点抽通路、配体受体、临床试验。」",
    ),
    help(
        "Turn this immunotherapy review into a knowledge graph. Focus on pathways, ligand-receptors, and trials.",
        "把这篇免疫治疗综述做成知识图谱，重点抽通路、配体受体、临床试验。",
    ),
    help(
        "I will extract and dedupe triples, then build a searchable graph colored by pathway, drug, and trial.",
        "先抽三元组并去重，再生成可搜索关系图，节点按通路、药物、试验着色。",
    ),
);

const BIO_DB: CapabilityIntro = intro(
    help(
        "Look up genes, proteins, datasets, structures, or papers in biology databases instead of a generic web search.\nClick the card and give a gene name, GSE, UniProt ID, or PMID, then say something as specific as: “Look up the main UniProt entry for TP53 and list two or three commonly used HCC GEO datasets”.",
        "要查基因、蛋白、数据集、结构或文献条目，且希望走 PubMed、GEO、UniProt、PDB 等库而不是泛网页。\n点卡片，直接报基因名、GSE 号、UniProt ID 或 PMID，第一句尽量写具体，例如：「查 TP53 在 UniProt 的主条目，并列 2–3 个常用肝癌相关 GEO 数据集」。",
    ),
    help(
        "Biology portals are scattered, so the same gene is looked up three times in three UIs.\nAccessions get mixed: a GSE, a GSM, and a UniProt ID land in the same note.\nA web search downloads the wrong series or an outdated record.\nCollaborators inherit a name instead of a canonical identifier.",
        "生物学数据库入口分散，同一个基因要在三个界面各查一遍。\nGSE、GSM、UniProt ID 写在同一条笔记里，对不上号。\n网页搜索容易下到错系列或过期记录。\n交接时只留下俗名，没有规范编号。",
    ),
    help(
        "Resolve a common name or ID to a canonical record | A gene symbol or messy accession becomes the official UniProt, GEO, or PDB entry.\nReturn accessions and links | You leave with IDs and URLs you can paste into a methods section.\nFewer trips through the wrong database | The lookup goes to the library that actually holds that record type.\nSample context attached | Dataset hits say what the samples are, not just the GSE number.",
        "用常用名或 ID 一次查到规范条目 | 基因俗名或混乱编号会落到官方 UniProt、GEO 或 PDB。\n带回 accession 与链接 | 离开时带着能写进方法的编号和网址。\n少在错误库里空转 | 按记录类型进该去的库，而不是泛网页碰运气。\n带上样本语境 | 数据集命中会说明样本是什么，不只给一个 GSE。",
    ),
    help(
        "Click the card and give a gene name, GSE, UniProt ID, or PMID\nStart with the actual task, for example: “Look up the main UniProt entry for TP53 and list two or three commonly used HCC GEO datasets.”\nIf a file, ID, or hard limit is missing, add it in the next message instead of restarting with a feature lecture.",
        "点卡片，直接报基因名、GSE 号、UniProt ID 或 PMID\n第一句直接进入任务，可参考：「查 TP53 在 UniProt 的主条目，并列 2–3 个常用肝癌相关 GEO 数据集。」\n若还缺文件、编号或硬限制，下一句直接补上，不必重开一轮功能介绍。",
    ),
    help(
        "Look up the main UniProt entry for TP53 and list two or three commonly used HCC GEO datasets.",
        "查 TP53 在 UniProt 的主条目，并列 2–3 个常用肝癌相关 GEO 数据集。",
    ),
    help(
        "I will return the canonical protein record and a checked GSE list, with sample type for each dataset.",
        "返回规范蛋白条目和经核对的 GSE 列表，并说明每个数据集的样本类型。",
    ),
);

const ACADEMIC_SEARCH_PRO: CapabilityIntro = intro(
    help(
        "Build a deduped metadata table or BibTeX before full-text reading or citation audits.\nClick the card and give keywords, then say something as specific as: “Search 2019–2024 “liquid biopsy” AND HCC, English research articles only. Dedupe into a literature matrix and BibTeX. Do not read full text yet”.",
        "开题或综述要先拉一张去重题录表或 BibTeX，还不需要读全文，也不做他引审计。\n点卡片，给关键词，可加年份或语种，第一句尽量写具体，例如：「检索 2019–2024 年 ‘liquid biopsy’ AND HCC，只要英文论著，去重后给文献矩阵和 BibTeX，先不要读全文」。",
    ),
    help(
        "The same paper appears in three libraries with three slightly different titles.\nEnglish and Chinese search rules do not match, so the matrix is incomplete.\nExports break: authors, years, or DOIs land in the wrong column.\nA request that mixes metadata, full-text reading, and citation counts never finishes.",
        "同一篇论文在三个库里各出现一次，题名还略有不同。\n中英检索口径不一致，题录表缺一块。\n导出后作者、年份、DOI 错列。\n把题录、全文精读和他引统计塞进一次请求，永远做不完。",
    ),
    help(
        "Parallel search of free libraries | OpenAlex, PubMed, arXiv, and Crossref are queried together and then deduped.\nLawful Chinese web search | Chinese hits go through a compliant web path, not a grey mirror.\nA deduped matrix or BibTeX | You get a table or .bib you can sort, not a pile of browser tabs.\nMetadata only, on purpose | Full-text reading and citation audits stay on their own cards.",
        "免费库并行检索 | OpenAlex、PubMed、arXiv、Crossref 一起查，再跨库去重。\n中文走合规网页检索 | 中文命中走合规路径，不走灰产镜像。\n去重后给矩阵或 BibTeX | 留下能排序的表或 .bib，而不是一堆标签页。\n故意只做题录 | 全文精读和他引审计留给各自的卡片。",
    ),
    help(
        "Click the card and give keywords\nAdd year or language if you have them\nSay you want metadata only\nStart with the actual task, for example: “Search 2019–2024 “liquid biopsy” AND HCC, English research articles only. Dedupe into a literature matrix and BibTeX. Do not read full text yet.”",
        "点卡片，给关键词，可加年份或语种\n说明只要题录，先不要读全文\n第一句直接进入任务，可参考：「检索 2019–2024 年 ‘liquid biopsy’ AND HCC，只要英文论著，去重后给文献矩阵和 BibTeX，先不要读全文。」",
    ),
    help(
        "Search 2019–2024 “liquid biopsy” AND HCC, English research articles only. Dedupe into a literature matrix and BibTeX. Do not read full text yet.",
        "检索 2019–2024 年 ‘liquid biopsy’ AND HCC，只要英文论著，去重后给文献矩阵和 BibTeX，先不要读全文。",
    ),
    help(
        "I will search in parallel, dedupe, and return a year/library matrix plus a .bib, marked as not citation-audited.",
        "并行检索并去重，按年/库输出矩阵和 .bib，并标明未做引用审计。",
    ),
);

const NATURE_ACADEMIC_SEARCH: CapabilityIntro = intro(
    help(
        "Verify references, build a MeSH strategy, count strict other-citations, and see who cited a paper, including high-profile citers.\nClick the card and give a DOI, title, or a reference list to audit, then say something as specific as: “How many strict other-citations does DOI 10.xxxx have? List high-profile citers and three references that may be wrong”.",
        "要核参考文献是否真实、做 MeSH、查严格他引、看谁引了这篇（含高影响引用者）。要的是引用关系，不是去重文献表。\n点卡片，给 DOI、题名或待核参考文献列表，第一句尽量写具体，例如：「这篇 DOI 10.xxxx 的严格他引有多少？列出高影响引用者和三条可能不实的参考文献」。",
    ),
    help(
        "A reference in the list has the wrong year, swapped authors, or a DOI that belongs to another paper.\nOther-citation counts are inflated by self-citation and you cannot defend the number.\nYou cannot see who picked the work up, or whether a high-profile group cited it.\nA literature-matrix card is used as if it could audit citations — it cannot.",
        "参考文献里年份错、作者对调，或 DOI 其实是另一篇。\n他引被自引稀释，报出去的数字经不起问。\n不知道工作被谁接走，有没有高影响团队引用。\n文献矩阵卡被当成引用审计用——它做不了这件事。",
    ),
    help(
        "Defensible checks of title, authors, year, and DOI | Each suspect item is checked field by field, not by a fuzzy “looks similar”.\nA strict other-citation count | Self-citations are stripped so the number can survive a committee question.\nHigh-profile citers named | You see who reused the work, not just a raw count.\nNot a replacement for the literature-matrix card | Metadata harvest stays on Academic Search Pro; this card audits identity and citation.",
        "核对题名、作者、年份和 DOI | 可疑条目按字段核，而不是「看起来像」。\n给出可辩护的他引口径 | 去掉自引，数字能经得起会上追问。\n标出高影响引用者 | 看到工作被谁接走，而不只是一个总数。\n不要用它替代文献矩阵检索 | 拉题录仍用学术检索；这张卡核身份和引用关系。",
    ),
    help(
        "Click the card and give a DOI, title, or a reference list to audit\nStart with the actual task, for example: “How many strict other-citations does DOI 10.xxxx have? List high-profile citers and three references that may be wrong.”\nIf a file, ID, or hard limit is missing, add it in the next message instead of restarting with a feature lecture.",
        "点卡片，给 DOI、题名或待核参考文献列表\n第一句直接进入任务，可参考：「这篇 DOI 10.xxxx 的严格他引有多少？列出高影响引用者和三条可能不实的参考文献。」\n若还缺文件、编号或硬限制，下一句直接补上，不必重开一轮功能介绍。",
    ),
    help(
        "How many strict other-citations does DOI 10.xxxx have? List high-profile citers and three references that may be wrong.",
        "这篇 DOI 10.xxxx 的严格他引有多少？列出高影响引用者和三条可能不实的参考文献。",
    ),
    help(
        "I will confirm the paper’s identity, count other-citations by the strict rule, and flag high-profile citers and suspect entries.",
        "先核文献身份，再按他引规则计数，并标出高影响引用与可疑条目。",
    ),
);

const NATURE_LITERATURE_PIPELINE: CapabilityIntro = intro(
    help(
        "Run search, scoring, deep-read, delivery, and archive as one package instead of a pile of links.\nClick the card and name the field, then say something as specific as: “Build a liver-fibrosis single-cell literature pack: last five years, public data required, a score table plus eight deep-read cards”.",
        "要把「检索→筛选→精读→交付→归档」一次做完，最后要的是一套材料包。\n点卡片，说明领域和怎样算够用，第一句尽量写具体，例如：「做一套肝纤维化单细胞文献包：近五年、必须有公开数据，最后给评分表加 8 张深读卡片」。",
    ),
    help(
        "Bookmark piles have no inclusion rule, so next week’s “good papers” are a different set.\nDeep-reads start before scoring, and the pack has no status.\nHandoff is a folder of PDFs with no card saying what was read and why.\nSearch, scoring, and delivery live in three chats that do not share state.",
        "收藏夹没有纳入标准，下周的「好论文」又是另一批。\n还没打分就开始精读，材料包没有状态。\n交接只是一堆 PDF，说不清读过什么、为什么留下。\n检索、评分、交付散在三段对话里，状态对不上。",
    ),
    help(
        "Inclusion and exclusion rules frozen first | The pack starts with a written rule, so later searches do not silently change the set.\nScored deep-reads | Only the papers that pass the score get a card; the rest stay in the matrix.\nAn archivable package you can reuse | Table plus cards can be reopened for a proposal or a new student.\nOne path instead of three chats | Search, score, read, and archive share the same state.",
        "纳入和排除标准先写死 | 材料包先有书面规则，后面检索不会悄悄换一批。\n评分后再精读 | 过线的才做卡片，其余留在矩阵里。\n交付物可归档复用 | 评分表加卡片能留给基金或新接手的人。\n一条路径而不是三段对话 | 检索、评分、精读、归档共用同一状态。",
    ),
    help(
        "Click the card and name the field\nDefine what “good enough” means\nSay the deliverable: brief, cards, or matrix\nStart with the actual task, for example: “Build a liver-fibrosis single-cell literature pack: last five years, public data required, a score table plus eight deep-read cards.”",
        "点卡片，说明领域和怎样算够用\n说明交付物形态：简报、卡片或矩阵\n第一句直接进入任务，可参考：「做一套肝纤维化单细胞文献包：近五年、必须有公开数据，最后给评分表加 8 张深读卡片。」",
    ),
    help(
        "Build a liver-fibrosis single-cell literature pack: last five years, public data required, a score table plus eight deep-read cards.",
        "做一套肝纤维化单细胞文献包：近五年、必须有公开数据，最后给评分表加 8 张深读卡片。",
    ),
    help(
        "I will freeze inclusion rules, search and score, then deep-read the chosen papers and archive the table and cards.",
        "先冻结纳入标准并检索打分，再精读入选篇目，归档评分表与卡片。",
    ),
);

const NATURE_DOWNLOADER: CapabilityIntro = intro(
    help(
        "You already have DOIs or titles and need PDFs through OA, institutional access, or publisher APIs.\nClick the card and give DOIs or titles, then say something as specific as: “Fetch these six DOIs through lawful routes. Mark a reason for any miss”.",
        "已有 DOI 或题名，要走 OA、机构权限或出版商 API 拿 PDF，不走灰产。\n点卡片，给 DOI 或题名，第一句尽量写具体，例如：「这 6 个 DOI 请走合法途径下 PDF，下不到的标原因」。",
    ),
    help(
        "Paywalls stop a batch download and nobody records why each DOI failed.\nGrey mirrors are tempting and unsafe for a lab that must stay lawful.\nThe file that lands is an author manuscript or the wrong version of record.\nA miss is reported as “failed” with no distinction between no OA and no institutional access.",
        "付费墙挡住一批下载，却没人记录每个 DOI 失败的原因。\n灰产镜像省事，但实验室必须走合法途径。\n下到的是作者稿或错误的正式版。\n失败只写「下不到」，分不清是无 OA 还是缺机构权限。",
    ),
    help(
        "Lawful routes only | OA, institutional access, and publisher APIs are tried; unofficial sources are not.\nClear note when a permission is missing | A miss says whether the lab lacks access or the paper has no OA.\nFewer wrong versions in the project folder | Hits are the version of record when the route allows it.\nA batch you can resume | Failed DOIs stay on a list instead of vanishing into chat.",
        "只走合法路由 | 只试 OA、机构权限和出版商接口，不走非官方源。\n失败时说清缺哪类权限 | 写明是实验室没权限，还是根本没有 OA。\n减少「下了个错版本」 | 路由允许时留下正式版，而不是作者稿冒充。\n批次可续跑 | 失败 DOI 留在清单里，不会消失在对话中。",
    ),
    help(
        "Click the card and give DOIs or titles\nSay whether you have institutional access\nStart with the actual task, for example: “Fetch these six DOIs through lawful routes. Mark a reason for any miss.”",
        "点卡片，给 DOI 或题名\n说明是否有机构访问\n第一句直接进入任务，可参考：「这 6 个 DOI 请走合法途径下 PDF，下不到的标原因。」",
    ),
    help(
        "Fetch these six DOIs through lawful routes. Mark a reason for any miss.",
        "这 6 个 DOI 请走合法途径下 PDF，下不到的标原因。",
    ),
    help(
        "I will try OA, institutional, and publisher APIs, file the hits, and say whether a miss is missing access or no OA.",
        "逐条尝试 OA、机构、出版商接口，成功的入库，失败的写明缺权限还是无 OA。",
    ),
);

const NATURE_PAPER_CARD: CapabilityIntro = intro(
    help(
        "Deep-read one paper into a card whose claims, methods, figures, and limits map back to the source.\nClick the card and give one PDF or DOI, then say something as specific as: “Make a deep-read card for this PDF. Tie Figure 2’s causal claim and its limits back to the text”.",
        "精读一篇，要主张、方法、图、局限都能对回原文，而不是读后感。\n点卡片，给一篇 PDF 或 DOI，第一句尽量写具体，例如：「给这篇 PDF 做深读卡片，重点对回 Figure 2 的因果主张和局限」。",
    ),
    help(
        "Reading notes miss figure numbers, so later citations point at the wrong panel.\nConditions and limits get mixed when you reuse a claim in related work.\nA collaborator cannot see which paragraph supported “Figure 2 is causal”.\nA highlight dump is not a card: claims, methods, and limits are one blob.",
        "笔记对不上图号，以后引用会指错面板。\n写相关工作时，条件和局限被记混。\n协作的人看不到「Figure 2 是因果」到底对回哪一段。\n高亮堆不是卡片：主张、方法、局限糊成一团。",
    ),
    help(
        "A structured Paper Card | Claims, methods, figures, and limits each have a slot, not a highlight dump.\nClaims with source anchors | Every strong sentence points back to a figure or paragraph.\nReusable for related work and review | The card can be dropped into a related-work draft or a referee note.\nEvidence boundaries called out | Over-causal wording is marked against the actual figure.",
        "结构化 Paper Card | 主张、方法、图、局限各有格子，不是一堆高亮。\n主张带定位 | 每句硬话都能指回图或段落。\n方便以后写相关工作和审稿 | 卡片能直接进相关工作或审稿备忘。\n证据边界单独标出 | 因果说过头的句子会对回实际的图。",
    ),
    help(
        "Click the card and give one PDF or DOI\nName the claims you care about\nStart with the actual task, for example: “Make a deep-read card for this PDF. Tie Figure 2’s causal claim and its limits back to the text.”",
        "点卡片，给一篇 PDF 或 DOI\n说明关注的主张\n第一句直接进入任务，可参考：「给这篇 PDF 做深读卡片，重点对回 Figure 2 的因果主张和局限。」",
    ),
    help(
        "Make a deep-read card for this PDF. Tie Figure 2’s causal claim and its limits back to the text.",
        "给这篇 PDF 做深读卡片，重点对回 Figure 2 的因果主张和局限。",
    ),
    help(
        "I will extract claims, methods, figures, and limits, each anchored to the source, and call out Figure 2’s evidence boundary.",
        "抽出主张、方法、图、局限，每条标回原文位置，并单独标出 Figure 2 的证据边界。",
    ),
);

const NATURE_READER: CapabilityIntro = intro(
    help(
        "Read original and translation side by side, with figures near first mention and clickable anchors — not the full paper dumped into chat.\nClick the card and give a PDF or DOI, then say something as specific as: “Turn this PDF into a bilingual reader. Keep figures with the section that first mentions them. Do not paste the full text into chat”.",
        "要边看原文边看译文，图出现在首次提及附近，并且能点回出处；不要把全文糊进聊天。\n点卡片，给 PDF 或 DOI，第一句尽量写具体，例如：「把这篇 PDF 做成中英对照阅读器，图跟章节走，不要把全文贴回对话」。",
    ),
    help(
        "Machine translation does not stay aligned with the original paragraphs.\nFigures detach from the section that first mentions them.\nPasting the full paper into chat blows up the thread and loses layout.\nYou cannot click from a translated sentence back to the source span.",
        "机翻和原文段落对不齐，左右没法对照。\n图离开了首次提及它的那一节。\n全文糊进聊天会撑爆对话，版式也没了。\n从译文句子点不回原文位置。",
    ),
    help(
        "An openable reader.html | The paper becomes a local bilingual report, not a chat dump.\nAligned columns | Original and translation stay on the same paragraph.\nClickable source anchors | A sentence in the translation can jump back to the source span.\nFigures stay with first mention | Panels sit next to the section that introduces them.",
        "生成可打开的 reader.html | 论文变成本地中英对照报告，而不是聊天长文。\n左右对照 | 原文和译文停在同一段，读的时候可以对着看。\n来源锚点可点 | 译文里的句子能跳回原文位置。\n图跟首次提及走 | 面板出现在介绍它的那一节旁边。",
    ),
    help(
        "Click the card and give a PDF or DOI\nAsk for a bilingual HTML report\nStart with the actual task, for example: “Turn this PDF into a bilingual reader. Keep figures with the section that first mentions them. Do not paste the full text into chat.”",
        "点卡片，给 PDF 或 DOI\n说明要中英对照 HTML 报告\n第一句直接进入任务，可参考：「把这篇 PDF 做成中英对照阅读器，图跟章节走，不要把全文贴回对话。」",
    ),
    help(
        "Turn this PDF into a bilingual reader. Keep figures with the section that first mentions them. Do not paste the full text into chat.",
        "把这篇 PDF 做成中英对照阅读器，图跟章节走，不要把全文贴回对话。",
    ),
    help(
        "I will write reader.html with side-by-side columns and figure anchors, and return only the report path in chat.",
        "生成 reader.html：左右对照、图锚在首次提及处，对话里只回报告路径。",
    ),
);

const RESEARCH_GRAPH: CapabilityIntro = intro(
    help(
        "The project already has data, papers, outputs, and decisions; you need to see why a choice was made.\nClick the card to open the graph panel — this is not a new guided chat, then say something as specific as: “After the research graph is open, explain why data asset GSE123 links to that last negative conclusion”.",
        "项目里数据、文献、产物、决策已经不少，要回看「当时为什么这么做」。\n点卡片打开图谱面板，不是开新对话，第一句尽量写具体，例如：「打开研究图谱后，帮我解释数据资产 GSE123 为什么连到上次那个阴性结论」。",
    ),
    help(
        "Evidence for a decision is buried in an old chat and cannot be replayed.\nA successor cannot tell which dataset version supported a negative result.\nThe same GSE is mentioned in three places with no edge between them.\nYou reopen the project and cannot answer “why did we stop that experiment”.",
        "决策证据埋在旧对话里，无法回放。\n接手的人对不上「阴性结论」到底用了哪一版数据。\n同一个 GSE 在三处被提起，中间没有边。\n重新打开项目，说不清当时为什么停掉那个实验。",
    ),
    help(
        "A project relationship map | Data, papers, outputs, and decisions sit on one graph instead of in chat archaeology.\nDecisions linked back to data and papers | A conclusion has edges to the assets that justified it.\nFewer repeated experiments | You see that a path was already tried before you rerun it.\nHandoff that survives a new student | The next person can open the graph instead of rereading every thread.",
        "一张项目关系图 | 数据、文献、产物、决策在一张图上，不用考古聊天。\n决策能链回数据和文献 | 结论连着当初支撑它的资产。\n减少重复实验 | 能看见这条路已经走过，不必再跑一遍。\n换人也能接手 | 新来的人打开图谱，而不用通读全部对话。",
    ),
    help(
        "Click the card to open the graph panel — this is not a new guided chat\nAsk in chat only when you want a node or edge explained\nStart with the actual task, for example: “After the research graph is open, explain why data asset GSE123 links to that last negative conclusion.”",
        "点卡片打开图谱面板，不是开新对话\n若要助手解释某条边，再在对话里点名节点\n第一句直接进入任务，可参考：「打开研究图谱后，帮我解释数据资产 GSE123 为什么连到上次那个阴性结论。」",
    ),
    help(
        "After the research graph is open, explain why data asset GSE123 links to that last negative conclusion.",
        "打开研究图谱后，帮我解释数据资产 GSE123 为什么连到上次那个阴性结论。",
    ),
    help(
        "From the graph edges and the recorded notes, I will show how that dataset supports the decision and mark missing controls.",
        "根据图谱边和当时记录，说明该数据集如何支持那条决策，并标出缺失的对照。",
    ),
);

const FILES_LIBRARY: CapabilityIntro = intro(
    help(
        "Reopen scripts, tables, figures, or saved cells without scrolling through chat.\nClick the card to open the files and library panel, then say something as specific as: “Is that volcano-plot SVG still in the library? Recolor it from the same data”.",
        "要找回脚本、表、图、收藏的代码格，不想翻聊天。\n点卡片打开文件和收藏面板，第一句尽量写具体，例如：「收藏库里那张火山图 SVG 还在吗？用同一套数据帮我改配色」。",
    ),
    help(
        "A plot only exists as an attachment on an old reply and the path is gone.\nScripts and SVGs are renamed until nobody knows which file is current.\nYou redo discovery just to recolor a figure that already lives in the project.\nChat search cannot find a saved cell the way a library panel can.",
        "图只活在某次回复的附件里，路径已经找不到。\n脚本和 SVG 改名到没人知道哪份是当前版。\n只为改配色又把发现过程重跑一遍。\n聊天搜索找不到收藏的代码格，文件面板可以。",
    ),
    help(
        "Browse files and the library by project | Scripts, tables, figures, and saved cells open from one panel.\nReuse work without rediscovering it | Recolor or rerun from the stored file instead of hunting chat attachments.\nCurrent version is visible | The library shows what is actually in the project now.\nChat stays a conversation | Large artifacts live as files, not as buried attachments.",
        "按项目浏览文件与收藏 | 脚本、表、图、收藏的代码格从同一面板打开。\n再分析或再作图不用重跑发现过程 | 从已存文件改配色或重跑，不用翻聊天附件。\n当前版本看得见 | 面板显示项目里现在到底是哪一份。\n聊天继续当对话用 | 大产物以文件存在，而不是埋在旧回复里。",
    ),
    help(
        "Click the card to open the files and library panel\nStart with the actual task, for example: “Is that volcano-plot SVG still in the library? Recolor it from the same data.”\nIf a file, ID, or hard limit is missing, add it in the next message instead of restarting with a feature lecture.",
        "点卡片打开文件和收藏面板\n第一句直接进入任务，可参考：「收藏库里那张火山图 SVG 还在吗？用同一套数据帮我改配色。」\n若还缺文件、编号或硬限制，下一句直接补上，不必重开一轮功能介绍。",
    ),
    help(
        "Is that volcano-plot SVG still in the library? Recolor it from the same data.",
        "收藏库里那张火山图 SVG 还在吗？用同一套数据帮我改配色。",
    ),
    help(
        "I will locate the SVG in the library, then re-export a recolored version from the original data in the analysis session.",
        "先在收藏库定位该 SVG，再在分析会话里按原数据重导出配色版。",
    ),
);

const DEMO: CapabilityIntro = intro(
    help(
        "You have no API key yet, or you want to see how the workbench looks before sending a request.\nClick the card to open the Example project, then say something as specific as: “Do not call a model. Show me a complete research demo project”.",
        "还没配 API Key，或想先看工作台长什么样。\n点卡片打开示例项目，第一句尽量写具体，例如：「先不要调模型，给我看一个完整科研演示项目」。\n第一句可以写成：「先不要调模型，给我看一个完整科研演示项目。」",
    ),
    help(
        "An empty project has no obvious next click, so first-time users stall.\nYou do not want to spend API quota just to see how the workbench looks.\nA sales or teaching walkthrough needs a complete research story, not a blank chat.\nThere is no safe place to show Runs, files, and a paper trail without a key.",
        "空项目不知道下一步点哪里，第一次用的人会停住。\n只想看工作台长什么样，却不想为此烧掉额度。\n演示或带教需要一条完整科研故事，不是空白对话。\n没有 API Key 时，也得能看见 Runs、文件和证据链。",
    ),
    help(
        "Read-only bundled demos | A complete example project opens without sending a new model request.\nReplay without a new request | Traces, files, and panels can be walked through offline.\nFaster onboarding | New users see what “done” looks like before they configure a key.\nSafe for teaching | You can show the workbench without billing a live turn.",
        "只读示例轨迹 | 打开完整示例项目，不会发出新的模型请求。\n零请求回看 | 轨迹、文件和面板可以离线走一遍。\n降低上手成本 | 配 Key 之前就能看见「做完」长什么样。\n适合演示和带教 | 展示工作台不必消耗一次付费对话。",
    ),
    help(
        "Click the card to open the Example project\nStart with the actual task, for example: “Do not call a model. Show me a complete research demo project.”\nIf a file, ID, or hard limit is missing, add it in the next message instead of restarting with a feature lecture.",
        "点卡片打开示例项目\n第一句直接进入任务，可参考：「先不要调模型，给我看一个完整科研演示项目。」\n若还缺文件、编号或硬限制，下一句直接补上，不必重开一轮功能介绍。",
    ),
    help(
        "Do not call a model. Show me a complete research demo project.",
        "先不要调模型，给我看一个完整科研演示项目。",
    ),
    help(
        "I will open the Example project and point to the replayable traces. No new model request is sent.",
        "打开示例项目并指出可回放的轨迹，不会发出新的模型请求。",
    ),
);

const NATURE_STATISTICS: CapabilityIntro = intro(
    help(
        "Methods or results are near submission; p-values, CIs, effect sizes, and wording must match journal habits.\nClick the card and paste methods plus results, then say something as specific as: “Here are three results paragraphs. Target Nature Communications. Fix journal-style stats wording and list missing effect sizes and multiple-comparison notes”.",
        "方法或结果将投稿，要检查 p、CI、效应量、多重比较和期刊统计口径。\n点卡片，贴方法加结果，第一句尽量写具体，例如：「这是结果里三段统计表述，目标 Nature Communications，请按期刊习惯改，并列出仍缺的效应量和多重比较」。",
    ),
    help(
        "Results report p-values only, with no effect size or confidence interval.\nA CI is written backwards or a test is switched after seeing the data.\nExploratory looks are sold as confirmatory analyses.\nJournal-specific stats wording is wrong and reviewers notice first.",
        "结果只报 p，没有效应量和置信区间。\nCI 方向写反，或看完数据再换检验。\n探索性分析被写成确证结论。\n期刊统计口径不对，审稿人第一眼就抓住。",
    ),
    help(
        "Itemized flags on bad wording | Each illegal or misleading stats sentence is marked, not waved through.\nPaste-ready rewrites | You get replacement sentences in the target journal’s habit.\nFewer stats comments from reviewers | Hard failures are cleared before the editorial office.\nMissing pieces listed | Effect sizes and multiple-comparison notes still absent are listed as work.",
        "按条目标出不合格表述 | 不合法或误导的统计句会被标出，而不是放行。\n给出可粘贴的改法 | 按目标期刊习惯给出替换句。\n降低统计审稿意见 | 硬伤在编辑部看见之前先清掉。\n仍缺的项会列成待办 | 还没写的效应量、多重比较会变成清单。",
    ),
    help(
        "Click the card and paste methods plus results\nName the target journal\nStart with the actual task, for example: “Here are three results paragraphs. Target Nature Communications. Fix journal-style stats wording and list missing effect sizes and multiple-comparison notes.”",
        "点卡片，贴方法加结果\n说明目标期刊\n第一句直接进入任务，可参考：「这是结果里三段统计表述，目标 Nature Communications，请按期刊习惯改，并列出仍缺的效应量和多重比较。」",
    ),
    help(
        "Here are three results paragraphs. Target Nature Communications. Fix journal-style stats wording and list missing effect sizes and multiple-comparison notes.",
        "这是结果里三段统计表述，目标 Nature Communications，请按期刊习惯改，并列出仍缺的效应量和多重比较。",
    ),
    help(
        "I will check p, CI, and effect size sentence by sentence, rewrite the bad lines, and list the multiple-comparison notes still missing.",
        "逐句核对 p、CI、效应量，改不合格句，并列出还要补的多重比较说明。",
    ),
);

const STATS_ANALYSIS: CapabilityIntro = intro(
    help(
        "You have a table and need hypothesis tests, regression, or exploratory plots — plus an explanation of what the result means.\nClick the card and name the outcome, comparison groups, and file path, then say something as specific as: “The table is data/clinic.csv. Compare overall survival in high vs low expression with KM and Cox. Covariates: age and stage”.",
        "已有表，要做假设检验、回归或探索图，并解释「结果是什么意思」。\n点卡片，说清结局、比较组和表路径，第一句尽量写具体，例如：「表在 data/clinic.csv，比较高/低表达组总生存，先做 KM 和 Cox，协变量年龄、分期」。",
    ),
    help(
        "The wrong test is chosen for the outcome, so the p-value does not answer the question.\nConfounders stay out of the model and the effect looks larger than it is.\nThe plot and the written claim tell two different stories.\nThere is no script, so the analysis cannot be rerun after a column fix.",
        "结局选错检验，p 值回答的不是那个问题。\n混杂没进模型，效应被放大。\n图和文字结论是两张皮。\n没有脚本，改完一列就无法重跑。",
    ),
    help(
        "A method matched to the question | The test or model is chosen from the outcome and design, not from habit.\nReproducible scripts | You leave with code that can rerun after the table changes.\nEffect direction and limits explained | HR, assumptions, and what the result does not prove are written down.\nPlots that match the claim | The figure is the same comparison the text argues.",
        "按问题选方法 | 检验或模型按结局和设计选，而不是按习惯。\n给出可复现脚本 | 表改了一列之后，还能用同一份脚本重跑。\n解释效应方向与限制 | HR、假设、以及结果不能证明什么，都会写清。\n图和结论同一比较 | 图做的比较就是正文在争的那一个。",
    ),
    help(
        "Click the card and name the outcome, comparison groups, and file path\nStart with the actual task, for example: “The table is data/clinic.csv. Compare overall survival in high vs low expression with KM and Cox. Covariates: age and stage.”\nIf a file, ID, or hard limit is missing, add it in the next message instead of restarting with a feature lecture.",
        "点卡片，说清结局、比较组和表路径\n第一句直接进入任务，可参考：「表在 data/clinic.csv，比较高/低表达组总生存，先做 KM 和 Cox，协变量年龄、分期。」\n若还缺文件、编号或硬限制，下一句直接补上，不必重开一轮功能介绍。",
    ),
    help(
        "The table is data/clinic.csv. Compare overall survival in high vs low expression with KM and Cox. Covariates: age and stage.",
        "表在 data/clinic.csv，比较高/低表达组总生存，先做 KM 和 Cox，协变量年龄、分期。",
    ),
    help(
        "After checking columns and missingness I will run KM and Cox, explain the HR and assumptions, and give script plus plots.",
        "检查列与缺失后跑 KM/Cox，解释 HR 与假设，并给出脚本和图。",
    ),
);

const PYTHON_R: CapabilityIntro = intro(
    help(
        "Analyze in a live Python or R session so variables stay loaded and plots are editable SVG.\nClick the card and name the language and data path, then say something as specific as: “Read results/expr.csv in Python, run PCA, keep the objects, export SVG. Next I will color by batch”.",
        "要在活会话里连续分析，变量留下，图要可改 SVG。\n点卡片，指明语言和数据路径，第一句尽量写具体，例如：「用 Python 读 results/expr.csv，做 PCA，变量留着，图导出 SVG，我接着要按批次上色」。",
    ),
    help(
        "Every follow-up reloads the table, so a long analysis never accumulates state.\nPlots export as rasters that cannot be recolored for submission.\nWriting and analysis sit in different tools and the numbers drift.\nYou cannot say “keep the PCA object” because the session is gone.",
        "每次追问都重读表，长分析攒不下状态。\n图导出成位图，投稿时无法改配色。\n写作和分析不在同一处，数字会漂。\n想留下 PCA 对象，会话却已经没了。",
    ),
    help(
        "A persistent session | Python or R keeps variables loaded so the next ask does not start from zero.\nFollow-ups without reload | Recolor or subset from objects that already exist.\nSVG that can go into a submission figure | Plots leave as editable vectors, not screenshots.\nAnalysis stays next to writing | Numbers in the manuscript can be regenerated from the same session.",
        "会话状态保留 | Python 或 R 把变量留着，下一问不用从零开始。\n跟进提问不用重来 | 在已有对象上改配色或子集。\nSVG 可进投稿图 | 图是可改矢量，不是截图。\n分析和写作挨着 | 稿子里的数字能从同一会话再生成。",
    ),
    help(
        "Click the card and name the language and data path\nIn the first message, say which objects to keep\nStart with the actual task, for example: “Read results/expr.csv in Python, run PCA, keep the objects, export SVG. Next I will color by batch.”",
        "点卡片，指明语言和数据路径\n第一句就说清要留下的对象\n第一句直接进入任务，可参考：「用 Python 读 results/expr.csv，做 PCA，变量留着，图导出 SVG，我接着要按批次上色。」",
    ),
    help(
        "Read results/expr.csv in Python, run PCA, keep the objects, export SVG. Next I will color by batch.",
        "用 Python 读 results/expr.csv，做 PCA，变量留着，图导出 SVG，我接着要按批次上色。",
    ),
    help(
        "I will load the table in a live session, emit a PCA SVG, keep the objects, and only change the color mapping next.",
        "在活会话加载数据并出 PCA SVG，保留对象，下一步只改配色映射。",
    ),
);

const REMOTE_COMPUTE: CapabilityIntro = intro(
    help(
        "The local machine is not enough: you need SSH or WSL, a long run, or a remote terminal.\nClick the card to open Environment settings, then say something as specific as: “I already added the lab GPU box. Run train.py there as a long job and tell me how to watch Runs”.",
        "本机跑不动，要 SSH 或 WSL，要挂长任务或看远程终端。\n点卡片打开「环境」设置，第一句尽量写具体，例如：「我已经加好实验室 GPU 机，请把 train.py 作为长任务在那台机器跑，并告诉我怎么看 Runs」。",
    ),
    help(
        "Training or a queue job cannot fit inside a chat shell timeout.\nSSH keys and remote paths have nowhere safe to live.\nA GPU box is used once from a one-off command and never becomes a registered context.\nWhen the job fails, there is no Run record to harvest logs from.",
        "训练或排队任务塞不进对话里的 shell 超时。\n密钥和远程路径没有安全存放处。\nGPU 机被一条临时命令用过一次，从未登记成上下文。\n任务失败后，没有 Run 记录可以收获日志。",
    ),
    help(
        "Register machines in Environment settings | SSH and WSL hosts become reusable contexts, not one-off commands.\nLong jobs become Runs | Training is submitted as a Run you can monitor, not a stretched timeout.\nKeys stay out of SQLite | Secrets stay in the keyring path the workbench already uses.\nHarvest and failure logs have an entry point | You know where to watch progress and pick up outputs.",
        "进入环境设置登记机器 | SSH 和 WSL 变成可复用上下文，而不是一次性命令。\n长任务走 Runs | 训练作为可监控的 Run 提交，而不是拉长超时。\n密钥不进 SQLite | 密钥走工作台已有的钥匙串路径。\n监控和失败日志有入口 | 知道去哪看进度、收获产物。",
    ),
    help(
        "Click the card to open Environment settings\nAdd SSH or WSL, then submit long jobs in chat\nStart with the actual task, for example: “I already added the lab GPU box. Run train.py there as a long job and tell me how to watch Runs.”",
        "点卡片打开「环境」设置\n添加 SSH 或 WSL 后，再在对话里提交长任务\n第一句直接进入任务，可参考：「我已经加好实验室 GPU 机，请把 train.py 作为长任务在那台机器跑，并告诉我怎么看 Runs。」",
    ),
    help(
        "I already added the lab GPU box. Run train.py there as a long job and tell me how to watch Runs.",
        "我已经加好实验室 GPU 机，请把 train.py 作为长任务在那台机器跑，并告诉我怎么看 Runs。",
    ),
    help(
        "I will submit a Run in the registered SSH context and point to monitor, harvest, and failure-log entry points.",
        "在已登记的 SSH 上下文提交 Run，并说明监控、收获产物和失败日志入口。",
    ),
);

const STRUCTURE: CapabilityIntro = intro(
    help(
        "Fold proteins, dock ligands, or design sequences with tools such as AlphaFold, Boltz, DiffDock, and MPNN.\nClick the card and give a FASTA or PDB plus the task: fold, dock, or design, then say something as specific as: “Fold this FASTA with an AlphaFold-style workflow, then dock this small molecule into the predicted pocket. Log the parameters”.",
        "要折叠、对接或设计序列，常用 AlphaFold、Boltz、DiffDock、MPNN 一类工具。\n点卡片，给 FASTA 或 PDB，并说明任务类型：折叠、对接或设计，第一句尽量写具体，例如：「这条 FASTA 用 AlphaFold 类流程折叠，再把这个小分子对到预测口袋，参数写进日志」。",
    ),
    help(
        "Fold, dock, and design steps are remembered as chat advice and cannot be rerun.\nParameters live in a screenshot and the next person uses different flags.\nPredicted structures never land in the project folder.\nConfidence and pocket checks are skipped, then docking is treated as fact.",
        "折叠、对接、设计只活在对话建议里，无法重跑。\n参数在截图里，下一手用了另一套开关。\n预测结构从未进项目目录。\n没看置信度和口袋就开始对接，结果被当成事实。",
    ),
    help(
        "A runnable recipe | Fold, check confidence, then dock — as steps you can repeat, not as folklore.\nLogged commands and parameters | Flags and versions are written down for the next run.\nStructure files stored in the project | PDB or score files land next to the notes that requested them.\nConfidence before interpretation | Low-confidence regions are marked before anyone designs a ligand story.",
        "把任务说成可执行步骤 | 先折叠、看置信度、再对接，步骤可重复，不是口口相传。\n留下命令与参数 | 开关和版本写下来，下一跑还能用。\n结构文件进项目 | PDB 或打分文件和当时的笔记放在一起。\n先看置信度再解释 | 低置信区域先标出，再讲配体故事。",
    ),
    help(
        "Click the card and give a FASTA or PDB plus the task: fold, dock, or design\nStart with the actual task, for example: “Fold this FASTA with an AlphaFold-style workflow, then dock this small molecule into the predicted pocket. Log the parameters.”\nIf a file, ID, or hard limit is missing, add it in the next message instead of restarting with a feature lecture.",
        "点卡片，给 FASTA 或 PDB，并说明任务类型：折叠、对接或设计\n第一句直接进入任务，可参考：「这条 FASTA 用 AlphaFold 类流程折叠，再把这个小分子对到预测口袋，参数写进日志。」\n若还缺文件、编号或硬限制，下一句直接补上，不必重开一轮功能介绍。",
    ),
    help(
        "Fold this FASTA with an AlphaFold-style workflow, then dock this small molecule into the predicted pocket. Log the parameters.",
        "这条 FASTA 用 AlphaFold 类流程折叠，再把这个小分子对到预测口袋，参数写进日志。",
    ),
    help(
        "I will fold first and check confidence, then dock, and return structure files plus a reproducible parameter table.",
        "先跑折叠并检查置信度，再对接，输出结构文件和可复现参数表。",
    ),
);

const SINGLE_CELL: CapabilityIntro = intro(
    help(
        "Single-cell QC, integration, annotation, or scvi/scGPT-style workflows with a paper trail.\nClick the card and give an h5ad or 10x path plus the goal: annotation, DE, or integration, then say something as specific as: “data/pbmc.h5ad: standard QC and Leiden annotation. Put key parameters into a methods draft”.",
        "有单细胞数据，要做质控、整合、注释或 scvi/scGPT 类流程，并留步骤。\n点卡片，给 h5ad 或 10x 路径，以及目标：注释、差异或整合，第一句尽量写具体，例如：「data/pbmc.h5ad，按标准流程质控和 Leiden 注释，关键参数写进方法草稿」。",
    ),
    help(
        "QC thresholds change every week and nobody can reconstruct the object.\nAnnotation is a verbal label with no Leiden resolution or marker table.\nMethods text cannot be written because key parameters were never logged.\nIntegration and DE live in two notebooks that do not share barcodes.",
        "质控阈值每周在变，没人能重建对象。\n注释只是口头标签，没有 Leiden 分辨率和 marker 表。\n关键参数没记下，方法部分写不出来。\n整合和差异分析在两个笔记本里，条码对不上。",
    ),
    help(
        "Steps written as a reproducible pipeline | QC, reduction, clustering, and annotation are one logged path.\nInspectable intermediates | h5ad or Seurat objects stay in the project for the next ask.\nMethods text almost ready to paste | Key parameters become a paragraph, not a memory.\nAnnotation you can defend | Clusters come with markers and resolution, not just pretty colors.",
        "步骤写成可复现流程 | 质控、降维、分群、注释是一条留了日志的路径。\n中间对象可回看 | h5ad 或 Seurat 对象留在项目里，下一问还能用。\n方便写方法部分 | 关键参数变成可粘贴段落，而不是回忆。\n注释经得起问 | 分群带着 marker 和分辨率，不只是好看的颜色。",
    ),
    help(
        "Click the card and give an h5ad or 10x path plus the goal: annotation, DE, or integration\nStart with the actual task, for example: “data/pbmc.h5ad: standard QC and Leiden annotation. Put key parameters into a methods draft.”\nIf a file, ID, or hard limit is missing, add it in the next message instead of restarting with a feature lecture.",
        "点卡片，给 h5ad 或 10x 路径，以及目标：注释、差异或整合\n第一句直接进入任务，可参考：「data/pbmc.h5ad，按标准流程质控和 Leiden 注释，关键参数写进方法草稿。」\n若还缺文件、编号或硬限制，下一句直接补上，不必重开一轮功能介绍。",
    ),
    help(
        "data/pbmc.h5ad: standard QC and Leiden annotation. Put key parameters into a methods draft.",
        "data/pbmc.h5ad，按标准流程质控和 Leiden 注释，关键参数写进方法草稿。",
    ),
    help(
        "I will run QC, reduction, clustering, and annotation, save the objects, and give a paste-ready methods paragraph.",
        "跑质控→降维→分群→注释，保存对象，并给出可粘贴的方法段落。",
    ),
);

const NATURE_EXPERIMENT_LOG: CapabilityIntro = intro(
    help(
        "Bench photos, voice notes, and scribbles need to become a searchable Markdown log.\nClick the card, upload photos or dictate, and name the experiment and date, then say something as specific as: “These four gel photos and a voice note are today’s Western. Turn them into a Markdown log and flag missing conditions”.",
        "台上只有照片、语音、随手字，要变成可检索的 Markdown 日志。\n点卡片，上传照片或贴口述，说明实验名称和日期，第一句尽量写具体，例如：「这 4 张胶片和一段语音是今天的 Western，整理成 Markdown 日志，缺的条件标出来」。",
    ),
    help(
        "Bench photos and voice notes never become a dated record.\nMaterials, lots, and exposure times are missing when you write Methods.\nA repeat experiment cannot see what was actually loaded last Tuesday.\nFigures live in the camera roll, separate from the observations.",
        "台上的照片和语音永远变不成带日期的记录。\n写方法时才发现材料、批号、曝光时间没记。\n补实验时看不到上周二到底上了什么样。\n图在手机相册里，和观察记录是两套。",
    ),
    help(
        "Structured date, materials, steps, and observations | A Markdown log has slots a later Methods section can lift.\nFigures in the same log | Gel photos sit next to the conditions that produced them.\nEasier methods and repeat experiments | Missing lots and times are flagged while you still remember.\nSearchable later | You can find “Western, Tuesday” without scrolling the camera roll.",
        "结构化日期、材料、步骤、观察 | Markdown 日志有格子，以后写方法能直接搬。\n图文进同一日志 | 胶片和产生它的条件放在一起。\n方便以后写方法和补实验 | 缺的批号和时间趁还记得就标出来。\n过后能搜到 | 不用翻相册也能找到「周二的 Western」。",
    ),
    help(
        "Click the card, upload photos or dictate, and name the experiment and date\nStart with the actual task, for example: “These four gel photos and a voice note are today’s Western. Turn them into a Markdown log and flag missing conditions.”\nIf a file, ID, or hard limit is missing, add it in the next message instead of restarting with a feature lecture.",
        "点卡片，上传照片或贴口述，说明实验名称和日期\n第一句直接进入任务，可参考：「这 4 张胶片和一段语音是今天的 Western，整理成 Markdown 日志，缺的条件标出来。」\n若还缺文件、编号或硬限制，下一句直接补上，不必重开一轮功能介绍。",
    ),
    help(
        "These four gel photos and a voice note are today’s Western. Turn them into a Markdown log and flag missing conditions.",
        "这 4 张胶片和一段语音是今天的 Western，整理成 Markdown 日志，缺的条件标出来。",
    ),
    help(
        "I will write a log with materials, steps, and results, and list unclear exposure times and antibody lots separately.",
        "写成带材料、步骤、结果的日志，识别不清的曝光时间和抗体批号单独列出。",
    ),
);

const ACADEMIC_PAPER: CapabilityIntro = intro(
    help(
        "Plan, draft, revise, and format a full manuscript — not one polished paragraph.\nClick the card and give a topic or draft, then say something as specific as: “Here is a results outline for a Brief Communication. Lock IMRaD and draft Introduction and Discussion skeletons”.",
        "从题目到各节起草、改稿、排版，要一篇完整稿而不是单段润色。\n点卡片，给题目或草稿，第一句尽量写具体，例如：「这是结果提纲，目标 Brief Communication，帮我定 IMRaD 并起草 Introduction 和 Discussion 骨架」。",
    ),
    help(
        "The outline promises IMRaD, but the body is still a results dump.\nTone jumps between sections because each one was written on a different day.\nFormatting is thrashed by hand the night before submission.\nIntroduction and Discussion never lock to the same claim set.",
        "提纲写着 IMRaD，正文仍是结果堆。\n各节语气不统一，因为不是同一天写的。\n投稿前一晚还在手工改格式。\n引言和讨论锁定的不是同一组主张。",
    ),
    help(
        "Structure first, then prose | IMRaD and length are frozen before long paragraphs are drafted.\nSection-wise revision | Introduction and Discussion can be rewritten without exploding the results.\nA submission-ready skeleton | Headings, word budget, and missing citations are visible as a map.\nOne claim set across the paper | The same findings appear in the intro promise and the discussion close.",
        "先定结构再写 | 先冻结 IMRaD 和篇幅，再写长段落。\n按节改 | 引言和讨论能改，结果部分不必跟着炸开。\n排出可投稿骨架 | 标题、字数和还缺的引用一眼能看见。\n全文共用同一组主张 | 引言承诺的和讨论收束的是同一批发现。",
    ),
    help(
        "Click the card and give a topic or draft\nName the article type and target journal\nStart with the actual task, for example: “Here is a results outline for a Brief Communication. Lock IMRaD and draft Introduction and Discussion skeletons.”",
        "点卡片，给题目或草稿\n说明文体（论著或短文）和目标期刊\n第一句直接进入任务，可参考：「这是结果提纲，目标 Brief Communication，帮我定 IMRaD 并起草 Introduction 和 Discussion 骨架。」",
    ),
    help(
        "Here is a results outline for a Brief Communication. Lock IMRaD and draft Introduction and Discussion skeletons.",
        "这是结果提纲，目标 Brief Communication，帮我定 IMRaD 并起草 Introduction 和 Discussion 骨架。",
    ),
    help(
        "I will freeze the outline and length, draft those two sections, and mark the key citations still missing.",
        "先冻结大纲与字数，再起草两节，并标出还缺的关键引用。",
    ),
);

const ACADEMIC_PIPELINE: CapabilityIntro = intro(
    help(
        "Research, write, review, and revise as one path instead of editing a single section each time.\nClick the card and say which stage you are on and what the final manuscript should be, then say something as specific as: “Continue from the existing literature matrix: draft, one mock review, then a revision. Track state”.",
        "要把调研、写作、审阅、修改放在同一条流水线，而不是每次只改一节。\n点卡片，说明现在停在哪一阶段、最终要哪类稿，第一句尽量写具体，例如：「从已有文献矩阵往下走：写初稿、做一轮模拟审稿、再改一版，状态请跟踪」。",
    ),
    help(
        "Literature, draft, review, and revision live in four chats with no shared state.\nA mock review is run on yesterday’s file while today’s draft moved on.\nHandoff cannot say which stage is current or what the next artifact is.\nLiterature holes are found only after the discussion is already written.",
        "调研、初稿、审阅、修改散在四段对话，没有共享状态。\n模拟审稿对着昨天的文件，今天的草稿已经往前走了。\n交接时说不清停在哪一阶段、下一份产物是什么。\n文献缺口要到讨论写完才被发现。",
    ),
    help(
        "End-to-end progress | Search, draft, review, and revise share one pipeline state.\nHandoff artifacts at each stage | Every stage leaves a file and a next step, not a verbal “we’re writing”.\nFewer late literature holes | The matrix stays attached until the revision is done.\nNo stop inside a single section | The path does not die after polishing one paragraph.",
        "端到端推进 | 检索、起草、审阅、修改共用一条流水线状态。\n阶段产物可交接 | 每阶段留下文件和下一步，而不是口头「我们在写」。\n减少「写完才发现文献缺口」 | 文献矩阵一直挂到改稿结束。\n不会在单节里停住 | 不会润色完一段就当整条路走完。",
    ),
    help(
        "Click the card and say which stage you are on and what the final manuscript should be\nStart with the actual task, for example: “Continue from the existing literature matrix: draft, one mock review, then a revision. Track state.”\nIf a file, ID, or hard limit is missing, add it in the next message instead of restarting with a feature lecture.",
        "点卡片，说明现在停在哪一阶段、最终要哪类稿\n第一句直接进入任务，可参考：「从已有文献矩阵往下走：写初稿、做一轮模拟审稿、再改一版，状态请跟踪。」\n若还缺文件、编号或硬限制，下一句直接补上，不必重开一轮功能介绍。",
    ),
    help(
        "Continue from the existing literature matrix: draft, one mock review, then a revision. Track state.",
        "从已有文献矩阵往下走：写初稿、做一轮模拟审稿、再改一版，状态请跟踪。",
    ),
    help(
        "I will move through pipeline stages, leave an artifact and a next step at each one, and not stop inside a single section.",
        "按流水线阶段推进，每阶段留下产物和下一步，不在单节里停住。",
    ),
);

const ACADEMIC_PAPER_REVIEWER: CapabilityIntro = intro(
    help(
        "Multi-role journal-style review before you submit: methods, statistics, and overclaim.\nClick the card and paste or upload the full manuscript, then say something as specific as: “Review this as Cell Reports with multiple roles. Give the three most fatal comments on methods, stats, and claims”.",
        "投稿前要多角色挑刺：方法、统计、主张是否过头。\n点卡片，贴或上传全稿，第一句尽量写具体，例如：「按 Cell Reports 口径多角色审这篇，分别出方法、统计、主张三条最致命意见」。",
    ),
    help(
        "Author-blind spots survive because only one polite reader sees the draft.\nComments arrive as vibes instead of referee-form, section-pointed notes.\nMethods, statistics, and overclaim are mixed into one undirected paragraph.\nYou cannot turn the review into a must-fix checklist.",
        "只有一位客气的读者看过，作者盲区还在。\n意见是感觉，不是审稿格式、对着章节的条目。\n方法、统计、主张过头糊在同一段里。\n审完变不成「必须改」清单。",
    ),
    help(
        "Multiple reviewer roles | Methods, stats, and claims are read as separate referee jobs.\nOverclaim, missing, and weak flags | Fatal items are named, not softened into “consider clarifying”.\nA revision checklist | Role reports collapse into an ordered must-fix list.\nJournal-calibrated tone | The comments sound like the venue you named, not a generic tutor.",
        "多角色意见 | 方法、统计、主张分开按审稿人职责读。\n标虚、缺、过头 | 致命项会被点名，而不是软化成「建议澄清」。\n可直接变成修改清单 | 角色报告收成有顺序的必改列表。\n按目标期刊口径 | 意见像你点名的那本杂志，而不是通用家教。",
    ),
    help(
        "Click the card and paste or upload the full manuscript\nName the target journal tier\nStart with the actual task, for example: “Review this as Cell Reports with multiple roles. Give the three most fatal comments on methods, stats, and claims.”",
        "点卡片，贴或上传全稿\n说明目标期刊档次\n第一句直接进入任务，可参考：「按 Cell Reports 口径多角色审这篇，分别出方法、统计、主张三条最致命意见。」",
    ),
    help(
        "Review this as Cell Reports with multiple roles. Give the three most fatal comments on methods, stats, and claims.",
        "按 Cell Reports 口径多角色审这篇，分别出方法、统计、主张三条最致命意见。",
    ),
    help(
        "I will write role-separated reports pointing to sections, then merge a must-fix checklist.",
        "分角色写审稿报告，每条指向章节，并汇总必须改的修正清单。",
    ),
);

const NATURE_WRITING: CapabilityIntro = intro(
    help(
        "Draft or rewrite sections in a tight, Nature-like style from results or an outline.\nClick the card and give result bullets or a Chinese outline, then say something as specific as: “From these five result bullets, draft a 180-word Nature-style Abstract. No slogans”.",
        "已有结果或提纲，要把章节写成紧、具体、像顶刊的英文。\n点卡片，给结果要点或中文提纲，第一句尽量写具体，例如：「根据这 5 条结果要点，起草一段 180 词的 Nature 风格 Abstract，不要口号」。",
    ),
    help(
        "Results exist as bullets, but the section does not read like a tight journal argument.\nA draft is long, hedged, and still missing the claim in the first sentence.\nFigure callouts and the prose do not lock to the same finding.\nYou need a rewrite, not a light copy-edit.",
        "结果还是条目，读起来不像顶刊那种紧的论证。\n草稿又长又绕，第一句仍没有主张。\n图注和正文锁的不是同一个发现。\n需要的是重写，不是轻轻改几个词。",
    ),
    help(
        "Tight section drafts | Results or an outline become Nature-like paragraphs with the claim up front.\nClaims locked to figures | Callouts point at the panel that actually shows the finding.\nRewrite, not perfume | Structure and verbs change; it is not a synonym pass.\nLength you can submit | The section respects the article type you named.",
        "按顶刊口径起草章节 | 结果或提纲变成主张前置的紧段落。\n主张锁到图 | 呼应的是真正展示该发现的面板。\n是重写不是喷香 | 结构和动词会改，不是换同义词。\n篇幅按文体来 | 章节尊重你点名的文章类型。",
    ),
    help(
        "Click the card and give result bullets or a Chinese outline\nName the section to write\nStart with the actual task, for example: “From these five result bullets, draft a 180-word Nature-style Abstract. No slogans.”",
        "点卡片，给结果要点或中文提纲\n指定要写的节\n第一句直接进入任务，可参考：「根据这 5 条结果要点，起草一段 180 词的 Nature 风格 Abstract，不要口号。」",
    ),
    help(
        "From these five result bullets, draft a 180-word Nature-style Abstract. No slogans.",
        "根据这 5 条结果要点，起草一段 180 词的 Nature 风格 Abstract，不要口号。",
    ),
    help(
        "I will write background, gap, approach, key numbers, and implication, each sentence mapped to your bullets.",
        "按背景—空白—做法—关键数—含义写 Abstract，每句能对回你给的要点。",
    ),
);

const NATURE_POLISHING: CapabilityIntro = intro(
    help(
        "Polish wording and tone in existing Chinese or English. Do not invent a new scientific story.\nClick the card, paste the paragraph, and say “polish only, do not change conclusions.”, then say something as specific as: “Polish this Discussion. Keep the causal claim. Cut empty “mechanistic insight.””.",
        "中英段落已经有了，要改措辞和语气，而不是重做科学故事。\n点卡片，贴段落并说明「只润色不改结论」，第一句尽量写具体，例如：「润色这段 Discussion，保持因果主张不变，去掉空泛的 mechanistic insight」。",
    ),
    help(
        "The science is fine, but the English or Chinese reads like a translation.\nTone flips between casual and stiff inside one paragraph.\nHedging and empty intensifiers hide the actual result.\nA bilingual draft has two voices that do not match.",
        "科学没问题，中英读起来却像翻译腔。\n同一段里语气一会儿随意一会儿生硬。\n套话和空洞强调把真正结果盖住。\n中英双语稿是两个声音，对不齐。",
    ),
    help(
        "Journal-like wording and tone | Sentences tighten toward the register of a top-journal paper.\nThe result stays visible | Polish does not invent new claims or delete numbers.\nOne voice across a bilingual draft | Chinese and English can be brought to the same tightness.\nPaste-ready paragraphs | You get text you can drop back into the manuscript.",
        "措辞和语气靠近高水平期刊 | 句子收到顶刊那种紧、克制的语域，而不是翻译腔。\n结果仍看得见 | 润色不新编主张，也不删数字。\n双语稿一个声音 | 中英可以收到同样的紧，读起来不像两个人写的。\n可粘回原稿 | 给出能直接贴回稿件的段落。",
    ),
    help(
        "Click the card, paste the paragraph, and say “polish only, do not change conclusions.”\nStart with the actual task, for example: “Polish this Discussion. Keep the causal claim. Cut empty “mechanistic insight.””\nIf a file, ID, or hard limit is missing, add it in the next message instead of restarting with a feature lecture.",
        "点卡片，贴段落并说明「只润色不改结论」\n第一句直接进入任务，可参考：「润色这段 Discussion，保持因果主张不变，去掉空泛的 mechanistic insight。」\n若还缺文件、编号或硬限制，下一句直接补上，不必重开一轮功能介绍。",
    ),
    help(
        "Polish this Discussion. Keep the causal claim. Cut empty “mechanistic insight.”",
        "润色这段 Discussion，保持因果主张不变，去掉空泛的 mechanistic insight。",
    ),
    help(
        "I will edit sentence by sentence, list deleted filler, and flag the line that still lacks data.",
        "逐句改措辞，列出被删的空话，并标出仍缺数据支持的一句。",
    ),
);

const HUMANIZER_ZH: CapabilityIntro = intro(
    help(
        "The text shows machine patterns. You need it to read like a person wrote it before submit or apply.\nClick the card, paste the text, and name the venue: paper, grant, or email, then say something as specific as: “This Chinese introduction is very AI-like. De-template it as paper prose. Do not add new citations”.",
        "中英文字明显像模型：套话、并列排比、假客观。投稿或申请前要去模板味。\n点卡片，贴原文，说明应用场景：论文、基金或邮件，第一句尽量写具体，例如：「这段中文引言 AI 味很重，按论文正文去模板，不要加新文献」。",
    ),
    help(
        "The draft is full of machine patterns: stacked abstractions, fake objectivity, and formula endings.\nA detector or a tired reader flags the text as AI before they reach the science.\nChinese or English “polish” made it smoother and more obviously synthetic.\nYou need a human cadence without changing the factual claims.",
        "草稿全是机器句式：抽象堆叠、假客观、公式化收尾。\n检测器或疲惫的读者还没看到科学，先判定这是 AI。\n所谓润色让它更顺，也更假。\n要人话节奏，但不能改事实主张。",
    ),
    help(
        "Machine patterns rewritten | Stock transitions and perfect parallel lists are broken on purpose.\nClaims stay put | Facts, numbers, and citations are not “creatively” changed.\nReads like a person | Cadence and specificity replace generic smoothness.\nWorks on Chinese or English | The same pass handles both languages.",
        "改掉明显的机器句式 | 套话连接和完美并列会被有意拆开。\n主张不动 | 事实、数字、引用不会被「创意」改掉。\n读起来像人写的 | 节奏和具体细节替掉那种假顺。\n中英都能处理 | 同一套改法覆盖两种语言。",
    ),
    help(
        "Click the card, paste the text, and name the venue: paper, grant, or email\nStart with the actual task, for example: “This Chinese introduction is very AI-like. De-template it as paper prose. Do not add new citations.”\nIf a file, ID, or hard limit is missing, add it in the next message instead of restarting with a feature lecture.",
        "点卡片，贴原文，说明应用场景：论文、基金或邮件\n第一句直接进入任务，可参考：「这段中文引言 AI 味很重，按论文正文去模板，不要加新文献。」\n若还缺文件、编号或硬限制，下一句直接补上，不必重开一轮功能介绍。",
    ),
    help(
        "This Chinese introduction is very AI-like. De-template it as paper prose. Do not add new citations.",
        "这段中文引言 AI 味很重，按论文正文去模板，不要加新文献。",
    ),
    help(
        "I will rewrite against common AI patterns, list the change types, and add no citations you did not provide.",
        "对照常见 AI 模式改写，列出改动类型，不新增未提供的引用。",
    ),
);

const NATURE_CITATION: CapabilityIntro = intro(
    help(
        "A paragraph has claims without references, or the citations are too old or too broad.\nClick the card, paste the paragraph, and name the field and whether you want primary studies only, then say something as specific as: “Add strict citations to this mechanism paragraph. Prefer primary studies after 2018. No textbooks”.",
        "段落里有主张但没文献，或引用太旧、太宽。\n点卡片，贴段落，说明学科和是否只要原始研究，第一句尽量写具体，例如：「这段机制讨论请补严格引用，优先 2018 年后原始研究，不要用教科书」。",
    ),
    help(
        "A mechanism paragraph makes strong claims with no primary reference.\nTextbook or review citations are used where an original paper is required.\nInserted citations do not actually support the sentence they sit in.\nYou cannot see which claims are still bare after the pass.",
        "机制段主张很硬，却没有原始文献。\n该引原始研究的地方引了教科书或综述。\n补上的引用其实撑不住它所在的那一句。\n改完仍看不出哪些主张还是裸的。",
    ),
    help(
        "Sentence-level citations | Each claim is searched and given a checkable item, or marked as still bare.\nPrimary evidence preferred | Original research is favored over textbooks when you ask for it.\nNo silent invention | Missing evidence is flagged instead of fabricating a reference.\nA list of leftover gaps | Claims that still lack a source stay on a punch list.",
        "按句补严格引用 | 每条主张都会检索并插入可核条目，或标成仍缺。\n优先原始研究 | 你要求时，原始论文优先于教科书。\n不默默编造 | 找不到证据就标出，不假造一条。\n剩下的缺口成清单 | 仍裸着的主张会留在待办里。",
    ),
    help(
        "Click the card, paste the paragraph, and name the field and whether you want primary studies only\nStart with the actual task, for example: “Add strict citations to this mechanism paragraph. Prefer primary studies after 2018. No textbooks.”\nIf a file, ID, or hard limit is missing, add it in the next message instead of restarting with a feature lecture.",
        "点卡片，贴段落，说明学科和是否只要原始研究\n第一句直接进入任务，可参考：「这段机制讨论请补严格引用，优先 2018 年后原始研究，不要用教科书。」\n若还缺文件、编号或硬限制，下一句直接补上，不必重开一轮功能介绍。",
    ),
    help(
        "Add strict citations to this mechanism paragraph. Prefer primary studies after 2018. No textbooks.",
        "这段机制讨论请补严格引用，优先 2018 年后原始研究，不要用教科书。",
    ),
    help(
        "I will search per sentence, insert checkable items, and mark claims that still lack primary evidence.",
        "按句检索并插入可核条目，标出仍找不到原始证据的主张。",
    ),
);

const NATURE_RESPONSE: CapabilityIntro = intro(
    help(
        "You have reviewer comments and a manuscript and need a point-by-point reply letter.\nClick the card and give the comments plus the manuscript or a change note, then say something as specific as: “Write the letter for these eight Reviewer 2 points. We added a sensitivity analysis for stats and will explain why we will not rerun the experiment”.",
        "已有审稿意见和稿子，要逐条 rebuttal。\n点卡片，同时给意见和稿，或改动说明，第一句尽量写具体，例如：「Reviewer 2 这 8 条，帮我写修回信：统计那条我们补了敏感性分析，方法那条我们解释为何不重做实验」。",
    ),
    help(
        "The paper changed, but the letter has no line pointers an editor can follow.\nReplies mix “we changed it”, “we explain”, and “we disagree” into one tone.\nReviewer 2’s eight points are answered in a narrative instead of one-to-one.\nA refused experiment is not explained, so it looks like the authors ignored it.",
        "正文改了，修回信却没有编辑跟得上的行号。\n「已改」「解释」「不同意」混在同一种语气里。\nReviewer 2 的八条被写成一篇叙事，而不是一对一。\n拒绝重做实验却不解释，看起来像作者无视意见。",
    ),
    help(
        "One reply per point | Each reviewer item gets its own answer with a pointer to the new text.\nDone, explain, or disagree labeled | The letter says what changed, what is explained, and what is declined.\nEditor-readable | Tone and structure help the editor route the file, not just the authors vent.\nPointers to new supplement and body | Added analyses are cited by location, not by memory.",
        "一条意见一封回复 | 每条审稿意见单独作答，并指向新正文。\n区分已改、解释、不同意 | 信里写清改了什么、解释什么、拒绝什么。\n方便编辑往下读 | 结构和语气帮编辑分流，而不是作者发泄。\n指向新增补充和正文 | 新分析按位置引用，不靠回忆。",
    ),
    help(
        "Click the card and give the comments plus the manuscript or a change note\nStart with the actual task, for example: “Write the letter for these eight Reviewer 2 points. We added a sensitivity analysis for stats and will explain why we will not rerun the experiment.”\nIf a file, ID, or hard limit is missing, add it in the next message instead of restarting with a feature lecture.",
        "点卡片，同时给意见和稿，或改动说明\n第一句直接进入任务，可参考：「Reviewer 2 这 8 条，帮我写修回信：统计那条我们补了敏感性分析，方法那条我们解释为何不重做实验。」\n若还缺文件、编号或硬限制，下一句直接补上，不必重开一轮功能介绍。",
    ),
    help(
        "Write the letter for these eight Reviewer 2 points. We added a sensitivity analysis for stats and will explain why we will not rerun the experiment.",
        "Reviewer 2 这 8 条，帮我写修回信：统计那条我们补了敏感性分析，方法那条我们解释为何不重做实验。",
    ),
    help(
        "I will answer point by point with pointers to the new supplement and body, in an editor-friendly tone.",
        "逐条写答复，指向新增补充图和正文位置，语气对编辑友好。",
    ),
);

const NATURE_REVIEWER: CapabilityIntro = intro(
    help(
        "A referee-style report before a high-impact submission.\nClick the card, upload a near-final draft, and name the target journal, then say something as specific as: “Write this as a Nature referee. List only issues that would cause reject, plus data we could add within a week”.",
        "投顶刊或高影响力刊前，要一份审稿人口吻的报告。\n点卡片，上传近终稿，说明目标期刊，第一句尽量写具体，例如：「按 Nature 审稿人写这份报告，只列会让编辑拒的问题和一周内能补的数据」。",
    ),
    help(
        "You cannot see the novelty, mechanism, or data hole an editor will stop on.\nA friendly read misses the reject-level issues a Nature referee would write.\nFixes are unprioritized, so a week of work goes to cosmetic comments.\nFigures and sections are not mapped, so the lab argues about the wrong panel.",
        "看不到编辑会停在哪条 Novelty、机制或数据空洞上。\n客气的通读漏掉 Nature 审稿人会写的拒稿级问题。\n修改没有优先级，一周工时花在皮毛意见上。\n图和章节没对上，组里在争打错的面板。",
    ),
    help(
        "Comments on novelty, mechanism, and data completeness | The report uses referee axes, not a summary of what the paper is about.\nPrioritized fixes you can act on | Fatal versus fixable-in-a-week items are split.\nMapped to figures and sections | Each comment points at a panel or heading.\nVenue-calibrated severity | A Nature-style read is harsher than a generic tutor pass.",
        "按 Novelty、机制、数据完整性出意见 | 报告用审稿人坐标，而不是复述论文讲了什么。\n给可执行修改优先级 | 致命项和一周内能补的项分开。\n对应到图和章节 | 每条意见指着面板或标题。\n按目标期刊的严厉程度 | Nature 口径比通用通读更狠。",
    ),
    help(
        "Click the card, upload a near-final draft, and name the target journal\nStart with the actual task, for example: “Write this as a Nature referee. List only issues that would cause reject, plus data we could add within a week.”\nIf a file, ID, or hard limit is missing, add it in the next message instead of restarting with a feature lecture.",
        "点卡片，上传近终稿，说明目标期刊\n第一句直接进入任务，可参考：「按 Nature 审稿人写这份报告，只列会让编辑拒的问题和一周内能补的数据。」\n若还缺文件、编号或硬限制，下一句直接补上，不必重开一轮功能介绍。",
    ),
    help(
        "Write this as a Nature referee. List only issues that would cause reject, plus data we could add within a week.",
        "按 Nature 审稿人写这份报告，只列会让编辑拒的问题和一周内能补的数据。",
    ),
    help(
        "I will produce a referee report with fatal vs fixable items mapped to figures and sections.",
        "出 referee report：致命项与可补项分开，并对应到图和章节。",
    ),
);

const NATURE_DATA: CapabilityIntro = intro(
    help(
        "Draft a Data Availability statement and a FAIR-style sharing plan an editor can accept.\nClick the card and list data types, whether they are human, and where you plan to host them, then say something as specific as: “I have bulk RNA-seq, a de-identified clinical table, and code. Target Genome Biology. Write the DAS and sharing steps”.",
        "要写 DAS、选仓库、把共享计划写成编辑能接受的一段。\n点卡片，列出数据种类、是否人类、打算放哪，第一句尽量写具体，例如：「有批量 RNA-seq、一张临床表（已脱敏）和代码，目标 Genome Biology，帮我写 DAS 和共享步骤」。",
    ),
    help(
        "The Data Availability statement is a vague “available upon request”.\nOmics, clinical tables, and code are dumped into one unsuitable repository.\nHuman fields that cannot go public are not listed, so ethics review fails later.\nSharing timing is missing: upon publication versus during review.",
        "数据可用性声明只是一句含糊的「来函可询」。\n组学、临床表和代码被塞进同一个不合适的仓库。\n不能公开的人类字段没列出，伦理审查后爆。\n共享时点没写：发表时还是审稿期间。",
    ),
    help(
        "A repository matched to each data type | Omics, clinical, and code each get a host an editor will accept.\nA checkable DAS | The statement names where and when, not “available upon request”.\nEthics and de-identification called out | Fields that still cannot go public stay on a list.\nSharing steps you can follow | Accession timing and who deposits what are written as actions.",
        "按数据类型推荐仓库 | 组学、临床、代码各有编辑接受的存放处。\n写出可检查的 DAS | 声明写清放哪、何时，而不是「来函可询」。\n标出伦理和脱敏前提 | 仍不能公开的字段留在清单上。\n共享步骤可执行 | 谁在何时交 accession，写成动作而不是愿望。",
    ),
    help(
        "Click the card and list data types, whether they are human, and where you plan to host them\nStart with the actual task, for example: “I have bulk RNA-seq, a de-identified clinical table, and code. Target Genome Biology. Write the DAS and sharing steps.”\nIf a file, ID, or hard limit is missing, add it in the next message instead of restarting with a feature lecture.",
        "点卡片，列出数据种类、是否人类、打算放哪\n第一句直接进入任务，可参考：「有批量 RNA-seq、一张临床表（已脱敏）和代码，目标 Genome Biology，帮我写 DAS 和共享步骤。」\n若还缺文件、编号或硬限制，下一句直接补上，不必重开一轮功能介绍。",
    ),
    help(
        "I have bulk RNA-seq, a de-identified clinical table, and code. Target Genome Biology. Write the DAS and sharing steps.",
        "有批量 RNA-seq、一张临床表（已脱敏）和代码，目标 Genome Biology，帮我写 DAS 和共享步骤。",
    ),
    help(
        "I will assign repositories and timing for omics, clinical, and code, give a paste-ready DAS, and list fields that still cannot go public.",
        "按组学、临床、代码分别指定仓库与时点，给出可粘贴 DAS，并列出还不能公开的字段。",
    ),
);

const NATURE_REF_VERIFIER: CapabilityIntro = intro(
    help(
        "Before submission or proofs, the reference list may have wrong years, authors, or DOIs.\nClick the card and paste a .bib, a list, or the end-of-paper references, then say something as specific as: “Check these 42 references. Suggest replacements for mismatches. Do not silently edit”.",
        "投稿或校样前，参考文献列表可能有错年、错作者、假 DOI。\n点卡片，贴 .bib、列表或稿末参考文献，第一句尽量写具体，例如：「核这 42 条参考文献，对不上的给出建议替换，不要静默改」。",
    ),
    help(
        "Years, authors, or DOIs in the list do not match the real paper.\nPreprint and published versions are mixed and the wrong one is cited.\nSilent edits would hide the mismatch from the corresponding author.\nProduction bounces the file for bibliography errors you could have caught.",
        "列表里的年份、作者或 DOI 对不上真正的那篇。\n预印本和正式版混用，引错了版本。\n静默修改会让通讯作者看不见对不上的条目。\n出版环节因参考文献错误退回，而这些本来能先核掉。",
    ),
    help(
        "Field-level checks of title, year, authors, and DOI | Each row is compared, not fuzzy-matched as “close enough”.\nUnmatched rows flagged | Mismatches stay visible until you confirm a replacement.\nNo silent rewrite | Suggested swaps are listed beside the original entries.\nFewer production bounce-backs | Bibliography errors are cleared before proofs.",
        "对题名、年份、作者和 DOI | 逐条比对，而不是「差不多就算」。\n标出对不上的条目 | 对不上的差异会留到你确认替换，不会被悄悄改掉。\n不静默改写 | 建议替换和原条目分开列。\n降低编辑部退回 | 校样前先清掉参考文献硬伤。",
    ),
    help(
        "Click the card and paste a .bib, a list, or the end-of-paper references\nStart with the actual task, for example: “Check these 42 references. Suggest replacements for mismatches. Do not silently edit.”\nIf a file, ID, or hard limit is missing, add it in the next message instead of restarting with a feature lecture.",
        "点卡片，贴 .bib、列表或稿末参考文献\n第一句直接进入任务，可参考：「核这 42 条参考文献，对不上的给出建议替换，不要静默改。」\n若还缺文件、编号或硬限制，下一句直接补上，不必重开一轮功能介绍。",
    ),
    help(
        "Check these 42 references. Suggest replacements for mismatches. Do not silently edit.",
        "核这 42 条参考文献，对不上的给出建议替换，不要静默改。",
    ),
    help(
        "I will check each entry, emit a diff table, keep originals until you confirm, and list suggested replacements separately.",
        "逐条核对并出差异表，建议替换分开列，原文条目保留到你确认。",
    ),
);

const NATURE_PAPER_TO_PATENT: CapabilityIntro = intro(
    help(
        "Turn inventive parts of a paper or lab notes into a Chinese invention-patent draft you can take to counsel.\nClick the card and give the paper or notes plus disclosure channels and dates, then say something as specific as: “Extract inventive points from this methods paper and draft a CN invention application. Do not write the claims too narrowly”.",
        "论文或实验笔记里有可申请点，要先出中国发明专利初稿再交给代理。\n点卡片，给论文或笔记，说明已公开渠道和截止日期，第一句尽量写具体，例如：「这篇方法论文请抽发明点，写成中国发明专利初稿，权要不要写太窄」。",
    ),
    help(
        "Inventive points stay buried in a methods paper the attorney has not read.\nClaims are drafted too narrowly, or so broadly they will not survive search.\nPrior publication dates are ignored, so novelty is already wounded.\nLab notes never become a specification an agent can file from.",
        "发明点埋在方法论文里，代理还没读过。\n权要写太窄，或宽到一检索就垮。\n已公开日期被忽略，新颖性已经受伤。\n实验笔记变不成代理能据以递交的说明书。",
    ),
    help(
        "Inventive points extracted | The draft lists what is actually new, with examples, before claim language.\nA specification and claims draft | You get a Chinese invention-application skeleton to take to counsel.\nLegal risks marked for an attorney | Prior disclosure and over-narrow claims are flagged, not hidden.\nNot a substitute for counsel | The file is a first draft, not a filed patent.",
        "抽出发明点 | 先列出真正新的点和实施例，再写权要语言。\n写成说明书和权利要求初稿 | 得到一份能交给代理的中国发明申请骨架。\n标明必须由代理审的法律风险 | 已公开和过窄权要会被标出，而不是藏起来。\n不能替代代理 | 这是初稿，不是已递交的专利。",
    ),
    help(
        "Click the card and give the paper or notes plus disclosure channels and dates\nStart with the actual task, for example: “Extract inventive points from this methods paper and draft a CN invention application. Do not write the claims too narrowly.”\nIf a file, ID, or hard limit is missing, add it in the next message instead of restarting with a feature lecture.",
        "点卡片，给论文或笔记，说明已公开渠道和截止日期\n第一句直接进入任务，可参考：「这篇方法论文请抽发明点，写成中国发明专利初稿，权要不要写太窄。」\n若还缺文件、编号或硬限制，下一句直接补上，不必重开一轮功能介绍。",
    ),
    help(
        "Extract inventive points from this methods paper and draft a CN invention application. Do not write the claims too narrowly.",
        "这篇方法论文请抽发明点，写成中国发明专利初稿，权要不要写太窄。",
    ),
    help(
        "I will list inventive points and examples, draft claims and the specification, and flag novelty risk from prior publication for counsel.",
        "列出发明点与实施例，起草权要和说明书，并提醒已发表对新颖性的影响需代理确认。",
    ),
);

const OFFICECLI: CapabilityIntro = intro(
    help(
        "You need a real .docx, .xlsx, or .pptx — not a screenshot or Markdown pretending to be Office.\nClick the card and name the file type, template, and output path, then say something as specific as: “Make an xlsx from this table, freeze the first row, add a growth column, and export a one-page PPT cover”.",
        "要真正的 Word、Excel 或 PPT，不是截图或 Markdown 冒充。\n点卡片，说清文件类型、模板要求和输出路径，第一句尽量写具体，例如：「按这张表做 xlsx，第一行冻结，补一列增长率，再导出一页 PPT 封面」。",
    ),
    help(
        "Chat returns Markdown or a screenshot that cannot be opened in Word or Excel.\nFormulas, frozen panes, and slide masters cannot live in a transcript.\nA cover letter, sheet, or deck has to be rebuilt by hand in desktop Office.\nThe file path is never written, so the next person cannot find the output.",
        "对话只给 Markdown 或截图，Word / Excel 打不开。\n公式、冻结窗格和幻灯片母版活不在聊天记录里。\n投稿信、汇总表或组会稿还得在桌面 Office 里手做。\n输出路径没写，下一手找不到文件。",
    ),
    help(
        "Real files on disk | A true .docx, .xlsx, or .pptx is written into the project.\nEditable in desktop Office | You can keep working in Word, Excel, or PowerPoint.\nUseful for letters, sheets, and decks | Cover letters, analysis tables, and one-pagers are first-class.\nOpenable paths returned | The reply names the file, not “I created a spreadsheet”.",
        "生成或改真实文件 | 项目里留下真正的 .docx / .xlsx / .pptx。\n可继续在桌面 Office 打开 | 能在 Word、Excel、PowerPoint 里接着改。\n适合投稿信、汇总表、组会稿 | 这些文件是一等公民，不是截图。\n回报可打开的路径 | 回复里是文件名，而不是「我做了个表」。",
    ),
    help(
        "Click the card and name the file type, template, and output path\nStart with the actual task, for example: “Make an xlsx from this table, freeze the first row, add a growth column, and export a one-page PPT cover.”\nIf a file, ID, or hard limit is missing, add it in the next message instead of restarting with a feature lecture.",
        "点卡片，说清文件类型、模板要求和输出路径\n第一句直接进入任务，可参考：「按这张表做 xlsx，第一行冻结，补一列增长率，再导出一页 PPT 封面。」\n若还缺文件、编号或硬限制，下一句直接补上，不必重开一轮功能介绍。",
    ),
    help(
        "Make an xlsx from this table, freeze the first row, add a growth column, and export a one-page PPT cover.",
        "按这张表做 xlsx，第一行冻结，补一列增长率，再导出一页 PPT 封面。",
    ),
    help(
        "I will write the xlsx and pptx into the project with OfficeCLI and return openable paths.",
        "用 OfficeCLI 写出 xlsx 和 pptx 到项目目录，并回报可打开的路径。",
    ),
);

const PUBLICATION: CapabilityIntro = intro(
    help(
        "Freeze the exact data, figures, and papers used for one manuscript version so you can audit later.\nClick the card to open the publication workspace. This is a panel, not a new skill chat, then say something as specific as: “Freeze Figure 3 and its stats script into the v0.3 evidence pack”.",
        "一稿一版要对死「用了哪些数据、图、文献」，并冻住以后核对。\n点卡片打开出版工作区。这是面板，不是新技能对话，第一句尽量写具体，例如：「把 Figure 3 和对应统计脚本冻进 v0.3 证据包」。",
    ),
    help(
        "Nobody can say which figure version and which stats script belong to manuscript v0.3.\nAfter submission, a raw table is missing and the pack cannot be audited.\nChat mentions of “Figure 3” do not freeze the file that was actually used.\nA new student rebuilds evidence that was already chosen, then silently swapped.",
        "没人说得清 v0.3 用的是哪版图、哪份统计脚本。\n投稿后发现原始表缺失，证据包无法审计。\n聊天里提「Figure 3」并没有冻住当时那份文件。\n新人把已经选定的证据重做一遍，还悄悄换掉。",
    ),
    help(
        "Versioned evidence pick | Figures, scripts, and tables are chosen for one manuscript version.\nA frozen pack you can audit | v0.3 stays inspectable after the chat has moved on.\nFewer post-submission scrambles | Missing raw files are listed while you can still find them.\nPanel, not a new skill chat | The publication workspace is a register, not another drafting thread.",
        "按版本挑选证据 | 图、脚本、表为某一稿版本被选中。\n冻结后可审计 | 对话往前走了，v0.3 仍能打开核对。\n减少投稿后翻盘 | 还缺的原始文件趁找得到就列出来。\n这是面板不是新技能对话 | 出版工作区是登记处，不是又开一稿。",
    ),
    help(
        "Click the card to open the publication workspace. This is a panel, not a new skill chat\nStart with the actual task, for example: “Freeze Figure 3 and its stats script into the v0.3 evidence pack.”\nIf a file, ID, or hard limit is missing, add it in the next message instead of restarting with a feature lecture.",
        "点卡片打开出版工作区。这是面板，不是新技能对话\n第一句直接进入任务，可参考：「把 Figure 3 和对应统计脚本冻进 v0.3 证据包。」\n若还缺文件、编号或硬限制，下一句直接补上，不必重开一轮功能介绍。",
    ),
    help(
        "Freeze Figure 3 and its stats script into the v0.3 evidence pack.",
        "把 Figure 3 和对应统计脚本冻进 v0.3 证据包。",
    ),
    help(
        "I will register that figure and script as v0.3 frozen and list any missing raw tables.",
        "在出版工作区登记该图与脚本，标记为 v0.3 冻结，并列出还缺的原始表。",
    ),
);

const NATURE_FIGURE: CapabilityIntro = intro(
    help(
        "You have a table or a clear plot idea and need a publication-style figure drawn or checked in Python or R.\nClick the card and give the data path and figure type: bar, scatter, or multi-panel, then say something as specific as: “Plot results/fig2.csv as a two-panel bar plus scatter at Nature single-column width. Deliver PDF and script”.",
        "有表或明确图意，要用 Python 或 R 画或检查能投稿的图。\n点卡片，给数据路径和图类型：柱、散点或多面板，第一句尽量写具体，例如：「用 results/fig2.csv 画双面板柱状+散点，按 Nature 单栏宽度，出 PDF 和脚本」。",
    ),
    help(
        "Default palettes and fonts get a journal bounce.\nA multi-panel figure has colliding labels and uneven column widths.\nThe plot exists as a screenshot with no script to change one axis.\nNature single-column width is guessed, then cropped in Illustrator at 1 a.m.",
        "默认配色和字体被期刊退回。\n多面板标签碰撞，栏宽也不齐。\n图只是截图，改一根轴没有脚本。\nNature 单栏宽度靠猜，凌晨还在 Illustrator 里裁。",
    ),
    help(
        "Journal-grade figures | Column width, type, and palette follow a submission contract, not a notebook default.\nEditable scripts | Python or R source is saved so one axis can change without redrawing by hand.\nCollision and alignment checks | Overlapping labels and drifted panels are caught before you send the PDF.\nPDF plus script delivered | The pair is what production actually wants.",
        "按投稿规范出图 | 栏宽、图种和色板跟投稿合同走，不是笔记本默认。\n可改脚本 | Python 或 R 源码留下，改一根轴不必手绘。\n检查碰撞与对齐 | 重叠标签和漂移面板在出 PDF 前就抓住。\n同时给 PDF 和脚本 | 生产环节要的就是这一对。",
    ),
    help(
        "Click the card and give the data path and figure type: bar, scatter, or multi-panel\nStart with the actual task, for example: “Plot results/fig2.csv as a two-panel bar plus scatter at Nature single-column width. Deliver PDF and script.”\nIf a file, ID, or hard limit is missing, add it in the next message instead of restarting with a feature lecture.",
        "点卡片，给数据路径和图类型：柱、散点或多面板\n第一句直接进入任务，可参考：「用 results/fig2.csv 画双面板柱状+散点，按 Nature 单栏宽度，出 PDF 和脚本。」\n若还缺文件、编号或硬限制，下一句直接补上，不必重开一轮功能介绍。",
    ),
    help(
        "Plot results/fig2.csv as a two-panel bar plus scatter at Nature single-column width. Deliver PDF and script.",
        "用 results/fig2.csv 画双面板柱状+散点，按 Nature 单栏宽度，出 PDF 和脚本。",
    ),
    help(
        "I will emit a column-width figure and an editable script, then run an alignment and collision check.",
        "生成符合栏宽的图和可改脚本，并做一次对齐和碰撞检查。",
    ),
);

const R_BIOINFO_FIGURE: CapabilityIntro = intro(
    help(
        "Expression or clinical tables need common bioinformatics plots in R: volcano, heatmap, PCA, survival.\nClick the card and give matrix or clinical paths plus the plots you want, then say something as specific as: “Two csv files, expr and clinic. Volcano and KM, grouped by subtype, one color system”.",
        "表达量或临床表要出火山图、热图、PCA、生存曲线等常见生信图。\n点卡片，说明矩阵或临床表路径和要的图，第一句尽量写具体，例如：「expr 和 clinic 两个 csv，画火山图和 KM，分组按 subtype，颜色统一」。",
    ),
    help(
        "Volcano, heatmap, PCA, and KM each use a different subtype color.\nGroup annotations are missing, so a reader cannot see what a column is.\nThe R script that made last month’s figure cannot be found.\nPanels cannot be assembled because exports have different resolutions.",
        "火山图、热图、PCA、KM 的亚型颜色各用一套。\n缺分组注释，读者看不出一列是什么。\n上个月出图的 R 脚本找不到了。\n导出分辨率不一致，拼不进同一张多面板。",
    ),
    help(
        "Convention-following R plots | Volcano, heatmap, PCA, and survival follow common bioinformatics grammar.\nReproducible scripts | The R path is saved next to the PDFs.\nPanel-ready outputs | One color system and comparable sizes make a later Figure 1 possible.\nClinical and expression kept in sync | Grouping comes from the same subtype column in both files.",
        "生信图专家按惯例出图 | 火山图、热图、PCA、生存曲线走常见生信语法。\n脚本可复现 | R 路径和 PDF 放在一起。\n便于拼图 | 同一套颜色和相近尺寸，以后才能拼 Figure 1。\n临床和表达对齐 | 分组来自两份文件里的同一列 subtype。",
    ),
    help(
        "Click the card and give matrix or clinical paths plus the plots you want\nStart with the actual task, for example: “Two csv files, expr and clinic. Volcano and KM, grouped by subtype, one color system.”\nIf a file, ID, or hard limit is missing, add it in the next message instead of restarting with a feature lecture.",
        "点卡片，说明矩阵或临床表路径和要的图\n第一句直接进入任务，可参考：「expr 和 clinic 两个 csv，画火山图和 KM，分组按 subtype，颜色统一。」\n若还缺文件、编号或硬限制，下一句直接补上，不必重开一轮功能介绍。",
    ),
    help(
        "Two csv files, expr and clinic. Volcano and KM, grouped by subtype, one color system.",
        "expr 和 clinic 两个 csv，画火山图和 KM，分组按 subtype，颜色统一。",
    ),
    help(
        "I will run the R bioinformatics path, unify the subtype palette, and return scripts plus PDFs.",
        "用 R 生信流程出图，统一分组色板，并给出脚本与 PDF。",
    ),
);

const AI_DRAWING: CapabilityIntro = intro(
    help(
        "You want a mechanism sketch, scene, or concept figure and have an image model configured.\nClick the card and describe the subject, style, and text that must not appear, then say something as specific as: “Draw a flat vector sketch of CAR-T recognizing a tumor cell. No garbled English”.",
        "要机制示意、流程图景、概念图，且已配置生图模型。\n点卡片，描述对象、风格、不要出现的文字，第一句尽量写具体，例如：「画一张 CAR-T 识别肿瘤细胞的示意，扁平矢量风，不要英文乱码」。",
    ),
    help(
        "You need a mechanism sketch for a talk and there is no illustrator on call.\nText in generated images comes back as garbled English.\nA first draft cannot be iterated because the prompt was never saved.\nA bitmap is used as a main figure when a vector redraw is still required for print.",
        "组会要机制示意，手头又没有插画。\n生成图里的文字变成英文乱码。\n提示词没留下，第一版无法迭代。\n位图被当成主图，印刷仍需要矢量重绘。",
    ),
    help(
        "A first draft from natural language | The configured image model draws from a written description.\nIterative edits | You can ask for a revision without starting the prompt from scratch.\nFine for talks | Journal-club and seminar sketches are the intended use.\nPrint still wants vectors | The card does not pretend a bitmap replaces a redrawn main figure.",
        "用自然语言出第一版 | 已配置的生图模型按文字描述出图。\n可迭代修改 | 改一版不必把提示词重写一遍。\n适合组会示意 | 组会和讨论班小图就是它的主场。\n投稿主图仍建议矢量重绘 | 这张卡不假装位图能替代主图重绘。",
    ),
    help(
        "Click the card and describe the subject, style, and text that must not appear\nStart with the actual task, for example: “Draw a flat vector sketch of CAR-T recognizing a tumor cell. No garbled English.”\nIf a file, ID, or hard limit is missing, add it in the next message instead of restarting with a feature lecture.",
        "点卡片，描述对象、风格、不要出现的文字\n第一句直接进入任务，可参考：「画一张 CAR-T 识别肿瘤细胞的示意，扁平矢量风，不要英文乱码。」\n若还缺文件、编号或硬限制，下一句直接补上，不必重开一轮功能介绍。",
    ),
    help(
        "Draw a flat vector sketch of CAR-T recognizing a tumor cell. No garbled English.",
        "画一张 CAR-T 识别肿瘤细胞的示意，扁平矢量风，不要英文乱码。",
    ),
    help(
        "I will call the configured image model and iterate one revision from your notes.",
        "调用已配置生图模型出图，并按你的修改意见迭代一版。",
    ),
);

const AI_MECHANISM_FIGURE: CapabilityIntro = intro(
    help(
        "Describe a pathway or drug mechanism and get a journal-style mechanism figure.\nThis card is a preview. Clicking the tile stays Coming Soon, then say something as specific as: “Draw a TGF-β liver-fibrosis mechanism figure in extracellular, membrane, and nuclear layers”.",
        "要把通路或药物机制画成顶刊常见机制图。\n这是预告卡，点卡片主体会提示正在开发，尚未开工，第一句尽量写具体，例如：「画 TGF-β 促肝纤维化的机制图，分细胞外、膜、核三层」。",
    ),
    help(
        "A pathway description has no journal-style layered figure yet — and this card is not wired.\nMolecules and arrows need extracellular, membrane, and nuclear grammar, not a clip-art pile.\nWhen it ships, the first draft must still be editable.\nUntil then, clicking the tile only shows Coming Soon.",
        "通路描述还没有顶刊那种分层机制图——这张卡也还没接通。\n分子和箭头需要细胞外、膜、核的语法，不是剪贴画堆。\n上线后第一稿仍必须能改。\n在此之前，点卡片只会提示正在开发。",
    ),
    help(
        "When shipped: journal mechanism grammar | Molecules and arrows will sit in extracellular, membrane, and nuclear layers.\nEditable layers | The draft is meant to stay changeable, not flatten to one bitmap.\nPreview only today | The tile will not start a run until the capability is wired.\nA target, not a promise of quality today | Use the card to learn the intended workflow.",
        "上线后按期刊机制图语法出图 | 分子和箭头会按细胞外、膜、核三层排。\n图层可继续改 | 目标是可改稿，而不是压成一张死图。\n今天只是预告 | 接通之前，点卡片不会开工。\n先看清工作流 | 用这张卡了解上线后怎么用，而不是假装现在能画。",
    ),
    help(
        "This card is a preview. Clicking the tile stays Coming Soon\nWhen shipped: describe the pathway and key molecules\nStart with the actual task, for example: “Draw a TGF-β liver-fibrosis mechanism figure in extracellular, membrane, and nuclear layers.”",
        "这是预告卡，点卡片主体会提示正在开发，尚未开工\n上线后：描述通路与关键分子即可\n第一句直接进入任务，可参考：「画 TGF-β 促肝纤维化的机制图，分细胞外、膜、核三层。」",
    ),
    help(
        "Draw a TGF-β liver-fibrosis mechanism figure in extracellular, membrane, and nuclear layers.",
        "画 TGF-β 促肝纤维化的机制图，分细胞外、膜、核三层。",
    ),
    help(
        "When this ships, I will place molecules and arrows in three layers and return an editable draft.",
        "上线后按三层排分子与箭头，并给出可编辑稿。",
    ),
);

const EDITABLE_FIGURE: CapabilityIntro = intro(
    help(
        "Turn a flattened mechanism figure into something you can still edit.\nThis card is a preview and is not wired yet, then say something as specific as: “On this mechanism figure, make Akt red and change the arrow to inhibition”.",
        "已有机制图位图，要变成还能改的对象。\n这是预告卡，点卡片主体尚未开工，第一句尽量写具体，例如：「这张机制图把 Akt 改成红色，箭头改成抑制」。\n第一句可以写成：「这张机制图把 Akt 改成红色，箭头改成抑制。」",
    ),
    help(
        "Changing one arrow on a flattened mechanism figure forces a full redraw.\nAkt color or an inhibition mark cannot be edited as objects today.\nThis card is a preview: the tile will not start a run yet.\nWhen it ships, the point is object-level edits, not another image generation.",
        "扁平机制图改一个箭头就要整图重绘。\n今天还不能把 Akt 颜色或抑制标记当成对象改。\n这是预告卡：点下去暂时不会开工。\n上线后要的是对象级编辑，不是再生成一张图。",
    ),
    help(
        "When shipped: object-level edits | Named molecules and arrows become things you can recolor or restyle.\nLess full redraws | One inhibition mark can change without regenerating the scene.\nPreview only today | Coming Soon is honest: there is no hidden start button.\nKeeps the original composition | The goal is edit, not a second guess at the layout.",
        "上线后支持对象级编辑 | 点名的分子和箭头变成能改颜色、改样式的对象。\n减少整图重绘 | 一个抑制标记能改，不必重生成整场。\n今天只是预告 | 「正在开发」是实话，没有暗门开工。\n尽量保住原构图 | 目标是改，不是再猜一版布局。",
    ),
    help(
        "This card is a preview and is not wired yet\nWhen shipped: upload the figure and name the elements to change\nStart with the actual task, for example: “On this mechanism figure, make Akt red and change the arrow to inhibition.”",
        "这是预告卡，点卡片主体尚未开工\n上线后：上传图并说明要改的元素\n第一句直接进入任务，可参考：「这张机制图把 Akt 改成红色，箭头改成抑制。」",
    ),
    help(
        "On this mechanism figure, make Akt red and change the arrow to inhibition.",
        "这张机制图把 Akt 改成红色，箭头改成抑制。",
    ),
    help(
        "When this ships, I will edit those objects on an editable layer without redrawing the whole figure.",
        "上线后在可编辑层改指定元素，不重绘整图。",
    ),
);

const BIOINFO_FIGURE_LAYOUT: CapabilityIntro = intro(
    help(
        "Arrange several bioinformatics plots into one submission-ready multi-panel figure.\nThis card is a preview and is not wired yet, then say something as specific as: “Combine these four plots as Figure 1: A volcano, B heatmap, C PCA, D KM”.",
        "多张生信图要拼成投稿多面板。\n这是预告卡，点卡片主体尚未开工，第一句尽量写具体，例如：「这四张图拼成 Figure 1，A 火山 B 热图 C PCA D KM」。",
    ),
    help(
        "Four plots sit in four files with four fonts and cannot become Figure 1.\nManual collage in PowerPoint drifts labels and column widths.\nThis card is a preview and will not arrange panels yet.\nWhen it ships, A/B/C/D labeling and unified width are the point.",
        "四张图四个文件四种字体，拼不成 Figure 1。\n在 PPT 里手工拼，标签和栏宽会漂。\n这是预告卡，现在还不会帮你排面板。\n上线后，A/B/C/D 标注和统一栏宽才是重点。",
    ),
    help(
        "When shipped: unified column width and labels | Separate volcano, heatmap, PCA, and KM become one submission panel.\nLess manual collage | Alignment is a layout job, not a late-night PowerPoint fight.\nPreview only today | The tile stays Coming Soon until the arranger is wired.\nPanel letters included | A/B/C/D are part of the deliverable, not an afterthought.",
        "上线后统一栏宽与标签 | 分开的火山图、热图、PCA、KM 会变成一张投稿拼图。\n减少手工拼 | 对齐是排版活，不是凌晨在 PPT 里对位。\n今天只是预告 | 排版器接通前，卡片保持「正在开发」。\n带上面板字母 | A/B/C/D 是交付物的一部分，不是事后手写。",
    ),
    help(
        "This card is a preview and is not wired yet\nWhen shipped: drop in several plots and name panels A/B/C\nStart with the actual task, for example: “Combine these four plots as Figure 1: A volcano, B heatmap, C PCA, D KM.”",
        "这是预告卡，点卡片主体尚未开工\n上线后：一次丢入多图并说明 A/B/C\n第一句直接进入任务，可参考：「这四张图拼成 Figure 1，A 火山 B 热图 C PCA D KM。」",
    ),
    help(
        "Combine these four plots as Figure 1: A volcano, B heatmap, C PCA, D KM.",
        "这四张图拼成 Figure 1，A 火山 B 热图 C PCA D KM。",
    ),
    help(
        "When this ships, I will align a submission multi-panel and return the collage with labels.",
        "上线后按投稿多面板对齐，并给出带 A/B/C/D 标注的拼图。",
    ),
);

const NATURE_PAPER2PPT: CapabilityIntro = intro(
    help(
        "A Chinese journal-club deck that follows the paper’s argument, not a wall of page screenshots.\nClick the card and give a PDF or DOI, plus duration and audience, then say something as specific as: “Make a 15-minute Chinese journal-club PPT from this methods paper. Few screenshots, more logic”.",
        "组会要讲一篇论文，要顺着论点做中文页，而不是页面截图墙。\n点卡片，给 PDF 或 DOI，说明时长和听众：组内或跨组，第一句尽量写具体，例如：「这篇方法论文做 15 分钟组会 PPT，中文，少放原文截图，多放逻辑」。",
    ),
    help(
        "Journal club becomes a wall of page screenshots with no argument.\nA 15-minute talk overflows because every figure is dumped in order.\nChinese presenter notes are missing, so the speaker reads English captions.\nThe deck cannot be edited because it is a flattened picture pile.",
        "组会变成原文截图墙，没有论点。\n每张图按顺序堆进去，15 分钟讲不完。\n没有中文讲解词，报告人只能念英文图注。\n片子是一层死图，没法改。",
    ),
    help(
        "Claim-structured slides | Pages follow question, method, key figures, and limits — not PDF page order.\nChinese presenter flow | The deck is meant to be spoken in Chinese for a lab meeting.\nAn editable deck | Shapes and text remain objects you can still change.\nFew screenshots on purpose | Logic slides beat a gallery of paper pages.",
        "按主张组织页面 | 按问题—方法—关键图—局限出页，而不是 PDF 页序。\n中文讲解向 | 片子是给组会中文讲的，不是念英文图注。\n可再改 | 形状和文字仍是对象，组会前还能继续改页。\n有意少放原文截图 | 逻辑页比把论文一页页贴上去更适合讲。",
    ),
    help(
        "Click the card and give a PDF or DOI, plus duration and audience\nStart with the actual task, for example: “Make a 15-minute Chinese journal-club PPT from this methods paper. Few screenshots, more logic.”\nIf a file, ID, or hard limit is missing, add it in the next message instead of restarting with a feature lecture.",
        "点卡片，给 PDF 或 DOI，说明时长和听众：组内或跨组\n第一句直接进入任务，可参考：「这篇方法论文做 15 分钟组会 PPT，中文，少放原文截图，多放逻辑。」\n若还缺文件、编号或硬限制，下一句直接补上，不必重开一轮功能介绍。",
    ),
    help(
        "Make a 15-minute Chinese journal-club PPT from this methods paper. Few screenshots, more logic.",
        "这篇方法论文做 15 分钟组会 PPT，中文，少放原文截图，多放逻辑。",
    ),
    help(
        "I will build pages as question, method, key figures, and limits, and generate an editable PPT.",
        "按问题—方法—关键图—局限出页，生成可编辑 PPT。",
    ),
);

const NATURE_IMAGE2PPT: CapabilityIntro = intro(
    help(
        "Rebuild an object-level editable PowerPoint from slide photos, screenshots, or a scanned PDF.\nClick the card, upload images or a scanned PDF, and say the page count and whether formulas need rebuild, then say something as specific as: “Rebuild these 12 photographed slides into an editable PPT. Title text must be editable”.",
        "只有幻灯片照片、截图或扫描 PDF，要还原成对象级可编辑 PPT。\n点卡片，上传图片或扫描 PDF，说明页数和是否要公式重排，第一句尽量写具体，例如：「这 12 页拍照幻灯片请还原成可编辑 PPT，标题文字一定要能改」。",
    ),
    help(
        "The only copy of a talk is a photographed slide deck or a scanned PDF.\nTitles are pixels, so a typo cannot be fixed without Photoshop.\nFormulas flatten and cannot be rebuilt as objects.\nRebuilding twelve pages from scratch takes longer than the next meeting allows.",
        "组会稿只剩拍照幻灯片或扫描 PDF。\n标题是像素，改错别字要上 Photoshop。\n公式被压扁，无法当对象重建。\n十二页从零重做，赶不上下场会。",
    ),
    help(
        "Recovered shapes and text | Pages are rebuilt as objects, not as one more flattened picture.\nFurther edits possible | Title text is meant to stay editable after reconstruction.\nFaster than rebuilding from scratch | Twelve photographed slides become a pptx you can still change.\nUncertain formulas flagged | Bad OCR on equations is listed instead of silently faked.",
        "尽量还原形状和文字 | 按页建成对象，而不是再压一层死图。\n可继续改内容 | 标题文字重建后仍应能改。\n比重做一稿快 | 十二页拍照稿变成还能改的 pptx。\n不确定的公式会标出 | 公式识别不清会列出，而不是默默假造。",
    ),
    help(
        "Click the card, upload images or a scanned PDF, and say the page count and whether formulas need rebuild\nStart with the actual task, for example: “Rebuild these 12 photographed slides into an editable PPT. Title text must be editable.”\nIf a file, ID, or hard limit is missing, add it in the next message instead of restarting with a feature lecture.",
        "点卡片，上传图片或扫描 PDF，说明页数和是否要公式重排\n第一句直接进入任务，可参考：「这 12 页拍照幻灯片请还原成可编辑 PPT，标题文字一定要能改。」\n若还缺文件、编号或硬限制，下一句直接补上，不必重开一轮功能介绍。",
    ),
    help(
        "Rebuild these 12 photographed slides into an editable PPT. Title text must be editable.",
        "这 12 页拍照幻灯片请还原成可编辑 PPT，标题文字一定要能改。",
    ),
    help(
        "I will reconstruct objects page by page, write a pptx, and flag uncertain formulas.",
        "按页重建对象，输出 pptx，并标出识别不确定的公式。",
    ),
);

const PPT_MASTER: CapabilityIntro = intro(
    help(
        "Build a real editable PowerPoint with masters, shapes, and charts from a topic or document. The skill downloads on first use.\nClick the card (first use installs the skill), then say something as specific as: “From this midterm outline, make a 10-page PPT with one master. Page 5 needs a bar chart”.",
        "从题目或材料做带母版、形状、图表的原生 PPT；第一次会下载技能。\n点卡片，第一次会先安装技能，第一句尽量写具体，例如：「根据这份中期汇报提纲做 10 页 PPT，统一母版，第 5 页要柱状图」。",
    ),
    help(
        "A midterm or teaching deck is still a Markdown outline with no master.\nCharts are screenshots, so a number change requires a new image.\nFirst use has no skill installed, and the card must download it.\nDesktop PowerPoint cannot open a “deck” that is only pictures.",
        "中期或课件还停留在没有母版的 Markdown 提纲。\n图是截图，改一个数字就要重出图。\n第一次用还没装技能，卡片必须先下载。\n桌面 PowerPoint 打不开只有图片的「片子」。",
    ),
    help(
        "Native PPT structure | Masters, shapes, and real chart objects land in a pptx.\nDesktop-editable | You continue in PowerPoint, not in a screenshot folder.\nUseful for reports and teaching | Midterm decks and class slides are the intended jobs.\nInstalls on first use | The skill is fetched once, then the same card builds decks.",
        "真 PPT 结构 | 母版、形状和真实图表对象写进 pptx。\n可在 PowerPoint 继续改 | 接着在桌面软件里改，不是对着截图文件夹。\n适合汇报和课 | 中期汇报和课件就是它的活。\n第一次用会安装技能 | 技能只拉一次，以后同一张卡出片子。",
    ),
    help(
        "Click the card (first use installs the skill)\nGive a topic or document and the page count\nStart with the actual task, for example: “From this midterm outline, make a 10-page PPT with one master. Page 5 needs a bar chart.”",
        "点卡片，第一次会先安装技能\n给题目或文档和页数\n第一句直接进入任务，可参考：「根据这份中期汇报提纲做 10 页 PPT，统一母版，第 5 页要柱状图。」",
    ),
    help(
        "From this midterm outline, make a 10-page PPT with one master. Page 5 needs a bar chart.",
        "根据这份中期汇报提纲做 10 页 PPT，统一母版，第 5 页要柱状图。",
    ),
    help(
        "After the skill is installed I will generate a mastered pptx with a real chart object on page 5.",
        "安装技能后生成带母版的 pptx，第 5 页用真实图表对象。",
    ),
);

const DATA_CLEANING: CapabilityIntro = intro(
    help(
        "Messy columns, mixed wide/long layout, or inconsistent dates and missingness — clean before analysis.\nClick the card, give a path or upload, and name the primary key and allowed values, then say something as specific as: “raw/clinic.xlsx has merged cells and mixed Chinese/English headers. Clean it into one analysis table and give a separate log”.",
        "表有乱码列、宽窄混乱、日期或缺失不统一，分析前要先整形。\n点卡片，给路径或上传表，说明主键和合法取值，第一句尽量写具体，例如：「raw/clinic.xlsx 有合并单元格和中英混列名，洗成一张分析表，日志单独给」。",
    ),
    help(
        "Merged cells and mixed Chinese/English headers make the table un-analyzable.\nWide and long layouts are tangled, so the primary key is not unique.\nDates and missingness codes differ by sheet and silently break a model.\nCleaning happens in place with no log, so you cannot undo a bad rename.",
        "合并单元格和中英混列名让表无法分析。\n宽表窄表缠在一起，主键不唯一。\n各表日期和缺失编码不同，模型会默默算错。\n原地清洗没有日志，错误重命名无法回退。",
    ),
    help(
        "Profile first, then clean | The specialist looks at the table before splitting, renaming, or deduping.\nA wash log | Every column change is written down so you can audit the wash.\nAn analysis-ready file | The output is one table a model can read.\nPrimary key and allowed values respected | Your constraints stay in the clean file.",
        "先探再洗 | 专家先看表，再拆分、重命名、去重。\n给出清洗日志 | 每列改动都记下，洗完能审计。\n输出可分析文件 | 结果是模型读得懂的一张表。\n尊重主键和合法取值 | 你定的约束会留在干净表里。",
    ),
    help(
        "Click the card, give a path or upload, and name the primary key and allowed values\nStart with the actual task, for example: “raw/clinic.xlsx has merged cells and mixed Chinese/English headers. Clean it into one analysis table and give a separate log.”\nIf a file, ID, or hard limit is missing, add it in the next message instead of restarting with a feature lecture.",
        "点卡片，给路径或上传表，说明主键和合法取值\n第一句直接进入任务，可参考：「raw/clinic.xlsx 有合并单元格和中英混列名，洗成一张分析表，日志单独给。」\n若还缺文件、编号或硬限制，下一句直接补上，不必重开一轮功能介绍。",
    ),
    help(
        "raw/clinic.xlsx has merged cells and mixed Chinese/English headers. Clean it into one analysis table and give a separate log.",
        "raw/clinic.xlsx 有合并单元格和中英混列名，洗成一张分析表，日志单独给。",
    ),
    help(
        "I will profile, then split, rename, and dedupe, and return a clean table plus a change log.",
        "先探查再拆分、重命名、去重，输出干净表和变更日志。",
    ),
);

const ENV_SETUP: CapabilityIntro = intro(
    help(
        "A new machine is missing Python, R, Node, sci, pixi, or officecli, so analysis or Office skills cannot run.\nClick the card to open runtime setup and wait until missing tools finish installing, then say something as specific as: “Finish setting up analysis runtimes on this Mac, then we will do Python analysis”.",
        "新电脑或缺 Python、R、Node、sci、pixi、officecli，分析或 Office 技能跑不起来。\n点卡片打开运行时安装界面，等待缺失项装完，第一句尽量写具体，例如：「先把这台 Mac 的分析环境补齐，再做 Python 分析」。",
    ),
    help(
        "A new Mac is missing Python, R, Node, or officecli, so analysis skills fail opaquely.\nEach tool is installed ad hoc and versions drift across the lab.\nError text does not say which runtime is missing.\nYou start an analysis session before the installer finishes.",
        "新 Mac 缺 Python、R、Node 或 officecli，分析技能报一堆看不懂的错。\n依赖各装各的，组里版本漂。\n报错不说缺的是哪一个运行时。\n安装还没结束就开始分析会话。",
    ),
    help(
        "One check-and-install path | Runtime setup scans and installs the missing analysis tools.\nFewer environment failures later | Skills start after the tools exist, not in the middle of a half-install.\nVisible missing pieces | You see which runtime was absent instead of a stack trace.\nThen the analysis session | Chat starts the real work only after setup reports done.",
        "统一检查并后台补齐 | 运行时安装界面扫描并补上缺失的分析工具。\n后续技能少踩环境坑 | 工具在了再开工，而不是装到一半就分析。\n缺谁看得见 | 看到缺的是哪个运行时，而不是一堆栈。\n装完再进分析会话 | 配置回报完成之后，对话才开始真正的活。",
    ),
    help(
        "Click the card to open runtime setup and wait until missing tools finish installing\nStart with the actual task, for example: “Finish setting up analysis runtimes on this Mac, then we will do Python analysis.”\nIf a file, ID, or hard limit is missing, add it in the next message instead of restarting with a feature lecture.",
        "点卡片打开运行时安装界面，等待缺失项装完\n第一句直接进入任务，可参考：「先把这台 Mac 的分析环境补齐，再做 Python 分析。」\n若还缺文件、编号或硬限制，下一句直接补上，不必重开一轮功能介绍。",
    ),
    help(
        "Finish setting up analysis runtimes on this Mac, then we will do Python analysis.",
        "先把这台 Mac 的分析环境补齐，再做 Python 分析。",
    ),
    help(
        "I will open runtime setup, install missing tools, and only then start the analysis session.",
        "打开环境配置并安装缺失运行时，完成后再进入分析会话。",
    ),
);

const PII_FIREWALL: CapabilityIntro = intro(
    help(
        "Chat may contain emails, phones, IDs, or lab keywords while the model is in the cloud.\nUse the switch on the card to turn it on or off, then say something as specific as: “Polish this acknowledgement that includes the PI email and a participant phone number”.",
        "对话可能含邮箱、手机、身份证或课题组内部词，但模型在云端。\n卡片开关控制开闭，第一句尽量写具体，例如：「把导师邮箱和受试者手机写进这段致谢里润色」。\n第一句可以写成：「把导师邮箱和受试者手机写进这段致谢里润色。」",
    ),
    help(
        "Consent forms, emails, and phone numbers are pasted into a cloud-model chat in the clear.\nA lab keyword or participant ID leaves the machine without a placeholder.\nThe reply you read still needs the real tokens, or it becomes unusable.\nResearch IDs such as GSE get over-redacted and the analysis breaks.",
        "知情同意书、邮箱、手机以明文贴进云端模型对话。\n课题组内部词或受试者编号未经占位就离开本机。\n你读到的回复如果不能还原，这段话就废了。\nGSE 这类科研编号被过度脱敏，分析跟着断。",
    ),
    help(
        "Outbound placeholders before the cloud sees the text | Emails, phones, IDs, and your dictionary terms are replaced on the way out.\nLocal restore in the reply you read | The on-screen answer puts the original tokens back on this machine.\nResearch IDs such as GSE left intact | Accessions you still need for analysis are not blindly scrubbed.\nA custom dictionary you can edit | Clicking the card opens the word list; this page only explains the switch.",
        "出站前替换成稳定代号 | 邮箱、手机、身份证和自定义词在出站前被替换。\n回复在本机还原 | 你看到的答案会在这台机器上还原原文。\n科研编号（如 GSE）默认不误伤 | 分析还要用的 accession 不会被一刀切掉。\n可自定义词表 | 点卡片打开词表；本页只解释开关。",
    ),
    help(
        "Use the switch on the card to turn it on or off\nClick the card to edit the dictionary\nThis help page only explains the capability\nStart with the actual task, for example: “Polish this acknowledgement that includes the PI email and a participant phone number.”",
        "卡片开关控制开闭\n点卡片打开词表编辑\n本页只介绍能力，不代替词表编辑\n第一句直接进入任务，可参考：「把导师邮箱和受试者手机写进这段致谢里润色。」",
    ),
    help(
        "Polish this acknowledgement that includes the PI email and a participant phone number.",
        "把导师邮箱和受试者手机写进这段致谢里润色。",
    ),
    help(
        "Outbound email and phone become EMAIL_1 and PHONE_1. The reply you see is restored on this machine.",
        "出站时邮箱和手机已替换成 EMAIL_1 和 PHONE_1，你看到的回复会还原原文。",
    ),
);

const JOURNAL_PRESCREEN: CapabilityIntro = intro(
    help(
        "After you have a target journal and a near-final manuscript, use this to check the paper against that journal’s author guidelines — format, ethics, length, figures, and required statements — before you click submit.\nIt is not another drafting pass. It is a preflight: the issues that usually explode only inside the submission system get listed with a fix, so you do not lose a week to desk rejection or a figure-format bounce.",
        "定了目标期刊、稿子也接近终稿之后，用它对照该刊作者须知做一轮预审：格式、伦理、字数、图数和必须声明，在点提交之前先过一遍。\n这不是再写一稿，而是投稿前检查。字数超了、缺伦理段、图格式不对这类问题，通常要到投稿系统才爆；这里会按条目标出，并给出改法，避免 desk reject 或补图耽误一周。",
    ),
    help(
        "Word count, abstract structure, or figure limits do not match the journal’s hard rules, and the submission site is the first place that says no.\nEthics, consent, competing-interest, or human/animal statements are missing a required paragraph.\nFigure resolution, file type, or legend style fails the checklist, so you reshoot or rebuild panels under time pressure.\nAuthor order, corresponding-author details, data-availability, or code-sharing text is too vague and comes back as an editorial query.",
        "字数、摘要结构或图表数量对不上该刊硬性限制，到投稿系统才被拦住。\n伦理、知情同意、利益冲突或人类/动物研究声明缺了一段，编辑直接 desk reject。\n图分辨率、格式或图注不符合须知，补图或重排版要耽误一周。\n作者顺序、通讯作者、数据可用性或代码共享写得太虚，审稿前被退回补材料。",
    ),
    help(
        "Itemized against the real guidelines | Checks the manuscript against that journal’s author instructions, not a generic “looks fine” impression.\nEvery flag comes with a fix | Does not stop at “this is wrong”: it says what to change, such as which statement to add or what dpi the figures need.\nFewer desk rejects | Clears the hard administrative failures before the editorial office sees the file.\nRe-run after you revise | After you fix length, figures, or ethics, you can prescreen the next version and confirm the list is clean.",
        "对照须知逐项打标 | 按目标期刊作者指南核对格式、伦理、篇幅和图，而不是凭印象改。\n每条问题带改法 | 不只说「不行」，会写清补哪段声明、图要做到多少分辨率、字数要压到哪。\n减少 desk reject | 把投稿系统里才爆的硬伤提前清掉，少一次被直接退稿。\n改完可再预审 | 调整字数、图和伦理项之后再跑一版，确认清单已经对齐。",
    ),
    help(
        "Click the Journal prescreen card to start a guided chat.\nGive the near-final manuscript (Word, PDF, or pasted text) and the target journal name. Paste the author-guidelines URL if you have it.\nSay what to emphasize: figures, ethics, length, or a full checklist.\nRead the issue list, apply the fixes, and send the revised files back if you want a second pass.",
        "点「论文预审」卡片，打开一场带引导的对话。\n把近终稿（Word、PDF 或正文）和目标期刊名发给它；若有作者须知链接，一并贴上。\n说明要重点看的项，例如图、伦理、字数，或按须知全检。\n按问题清单改完后，可以把修订稿再发回来做第二轮预审。",
    ),
    help(
        "Prescreen this manuscript against the eLife author guidelines. Please focus on figures and ethics, but still flag any hard length or statement failures. Give a checklist with a concrete fix for each item.",
        "对照 eLife 作者须知预审这篇近终稿。图和伦理两项重点看，但字数、声明这类硬伤也请标出来。请给问题清单，每一条都写具体改法。",
    ),
    help(
        "I will read the eLife author instructions first, then walk the manuscript item by item. You will get a checklist covering figure resolution and legends, ethics/consent language, and any length or required-statement failures, each with a specific fix — not just a pass/fail.",
        "先对照 eLife 作者须知，再按项检查稿件。你会拿到一份清单：图分辨率和图注、伦理/知情同意表述，以及字数或必填声明方面的硬伤，每条都带具体改法，而不是只说通过或不通过。",
    ),
);

const FIGURE_DUPLICATE_AUDIT: CapabilityIntro = intro(
    help(
        "A manuscript PDF or a folder of figure images needs a reuse check before submission or after a reviewer flags a panel.\nClick the card, then attach one PDF or name one image directory. Say something as specific as: “Audit figures 2–5 in this PDF for reused or flipped panels”.",
        "投稿前，或审稿人点名某张图之后，要用 PDF 或图目录做一轮图块查重。\n点卡片，附一份 PDF 或点名一个图片目录。第一句尽量写具体，例如：「查这份 PDF 的 Figure 2–5，看有没有复用或翻转后再用的图块」。",
    ),
    help(
        "The same blot or micrograph appears in two figures, sometimes cropped or flipped, and nobody compared the panels.\nA hash match is treated as proof of misconduct instead of a candidate for human review.\nA whole-PDF render includes captions and page furniture, so the matcher scores layout instead of the experiment.\nAn unreadable file is silently dropped, so coverage looks complete when it is not.",
        "同一张 blot 或镜下图出现在两张图里，有时被裁切或翻转，没人对过图块。\n哈希命中被当成不端证据，而不是给人复核的候选。\n整页渲染带着图注和页眉页脚，匹配器在对排版而不是实验图。\n读不出的文件被默默丢掉，覆盖率看起来完整，其实漏了。",
    ),
    help(
        "Candidates, not a verdict | Hashes and feature matches find possible reuse; the report does not call it misconduct.\nPanel-level evidence | Composite figures are split so the comparison is the experimental unit, not the whole page.\nCoverage you can check | Skipped and unreadable files stay in the accounting instead of disappearing.\nA folder you can reopen | Manifest, pair table, and visual evidence land in a new analysis directory.",
        "给候选，不下判决 | 哈希和特征匹配找出可能复用，报告不会写成不端结论。\n按图块比对 | 组合图会拆开，比的是实验单元，不是整页。\n覆盖率可核对 | 跳过和读不出的文件会记在账上，不会消失。\n结果可回看 | 清单、配对表和可视化证据写进新的分析目录。",
    ),
    help(
        "Click the card to start a guided chat with the Figure Duplicate Audit specialist\nAttach one PDF or name one image directory; do not ask it to scan the project\nGive a page range when the PDF is long\nStart with the actual task, for example: “Audit figures 2–5 in this PDF for reused or flipped panels.”",
        "点卡片，打开一场由「图片查重」专家引导的对话\n附一份 PDF 或点名一个图片目录，不要让它自行扫项目\nPDF 很长时写清页码范围\n第一句直接进入任务，可参考：「查这份 PDF 的 Figure 2–5，看有没有复用或翻转后再用的图块。」",
    ),
    help(
        "Audit figures 2–5 in this PDF for reused or flipped panels.",
        "查这份 PDF 的 Figure 2–5，看有没有复用或翻转后再用的图块。",
    ),
    help(
        "I will extract and split those figure pages, score candidate pairs, and write a cautious integrity report with visual evidence. A match is a candidate, not a finding of misconduct.",
        "会提取并拆开这些图页，给候选配对打分，并写出带可视化证据的谨慎报告。匹配只是候选，不是不端结论。",
    ),
);

const HANDWRITING_EXTRACT: CapabilityIntro = intro(
    help(
        "Handwritten lab notes or CRF photos need to become a table.\nSet the vision and calibration models in the gear first, then say something as specific as: “These three CRF pages: date as YYYY-MM-DD, dose in mg, yes/no. Extract a table and highlight uncertain cells”.",
        "实验本、CRF、表格是手写拍照，要进电子表。\n先在齿轮配好识图和校准模型，第一句尽量写具体，例如：「这 3 页 CRF，列分别是日期(YYYY-MM-DD)、剂量(mg)、是/否。请提取成表，不确定的标黄」。",
    ),
    help(
        "Handwritten CRF pages are typed by hand and digit columns flip 1 and 7.\nUncertain cells are silently guessed, then enter the analysis table as fact.\nColumn formats are never told to the model, so dates and doses parse wrong.\nVision and calibration models are still on default when you upload the photos.",
        "手写 CRF 靠人工誊，数字列的 1 和 7 会看反。\n不确定的格子被默默猜出来，再当事实进分析表。\n没告诉模型每列格式，日期和剂量就会拆错。\n上传照片时识图和校准模型还停在默认。",
    ),
    help(
        "A vision model plus a calibration model | Photos are read, then doubtful cells are checked by a second model.\nUncertain cells flagged | Low-confidence boxes are highlighted instead of guessed.\nColumn-format hints raise accuracy | Telling it YYYY-MM-DD and mg prevents a silent parse failure.\nA table you can open | The extract lands as a spreadsheet, not a paragraph of numbers.",
        "分析模型识图，校准模型核存疑格 | 先识图，再用第二个模型核拿不准的格子。\n不确定处打标 | 低置信格会标出，而不是猜完当事实。\n你补充列格式后准确率明显上升 | 写清 YYYY-MM-DD 和 mg，能避免默默拆错。\n输出能打开的表 | 提取结果是电子表，不是一段数字散文。",
    ),
    help(
        "Set the vision and calibration models in the gear first\nClick the card, upload photos, and describe each column’s format\nStart with the actual task, for example: “These three CRF pages: date as YYYY-MM-DD, dose in mg, yes/no. Extract a table and highlight uncertain cells.”",
        "先在齿轮配好识图和校准模型\n再点卡片上传照片，并说明每列格式\n第一句直接进入任务，可参考：「这 3 页 CRF，列分别是日期(YYYY-MM-DD)、剂量(mg)、是/否。请提取成表，不确定的标黄。」",
    ),
    help(
        "These three CRF pages: date as YYYY-MM-DD, dose in mg, yes/no. Extract a table and highlight uncertain cells.",
        "这 3 页 CRF，列分别是日期(YYYY-MM-DD)、剂量(mg)、是/否。请提取成表，不确定的标黄。",
    ),
    help(
        "I will extract and calibrate with both models, output a table, and list low-confidence cells separately.",
        "用双模型提取并校准，输出表，低置信格单独列出。",
    ),
);

const TOPIC_COACH: CapabilityIntro = intro(
    help(
        "You already have data or materials and need a few journal-fit topic candidates plus a next action.\nClick the card and inventory data, cohort, existing figures, and the target journal, then say something as specific as: “I have 80 paired transcriptomes with follow-up. I want a CAS zone-2 oncology journal. Give three scored topics”.",
        "手里已有数据、样本或方法，要找几个对准期刊的题目，并知道下一步。\n点卡片，盘点数据、队列、已有图表和目标期刊，第一句尽量写具体，例如：「我有 80 例配对转录组和随访，想投中科院 2 区肿瘤刊，给 3 个选题并打分」。",
    ),
    help(
        "Topic ideas are bigger than the cohort or ignore the figures you already have.\nYou cannot tell whether the next month should be another experiment or a draft.\nJournal fit is a feeling, not a scored shortlist.\nA fantasy title survives because nobody mapped it onto the actual data.",
        "题目比队列大，或忽略你已经有的图。\n说不清下个月该补实验还是开写。\n期刊匹配是感觉，不是打过分的短名单。\n没人把标题映射到真实数据，空中楼阁就留下来了。",
    ),
    help(
        "Scored candidates from what you already have | Topics are rated against the cohort, figures, and target zone you named.\nA concrete next action | Each candidate says whether to write, analyze, or run one more assay.\nFewer fantasy topics | Titles that the data cannot support are dropped.\nJournal-fit is explicit | CAS zone or venue constraints stay in the score, not in a sidebar wish.",
        "从已有材料出发打分 | 按你点名的队列、已有图和目标分区给选题打分。\n给可执行下一步 | 每个候选会说该写、该分析，还是再补一个实验。\n避免空中楼阁选题 | 数据撑不住的标题会被丢掉。\n期刊匹配写在分数里 | 中科院分区或刊物约束进评分，而不是旁边许愿。",
    ),
    help(
        "Click the card and inventory data, cohort, existing figures, and the target journal\nStart with the actual task, for example: “I have 80 paired transcriptomes with follow-up. I want a CAS zone-2 oncology journal. Give three scored topics.”\nIf a file, ID, or hard limit is missing, add it in the next message instead of restarting with a feature lecture.",
        "点卡片，盘点数据、队列、已有图表和目标期刊\n第一句直接进入任务，可参考：「我有 80 例配对转录组和随访，想投中科院 2 区肿瘤刊，给 3 个选题并打分。」\n若还缺文件、编号或硬限制，下一句直接补上，不必重开一轮功能介绍。",
    ),
    help(
        "I have 80 paired transcriptomes with follow-up. I want a CAS zone-2 oncology journal. Give three scored topics.",
        "我有 80 例配对转录组和随访，想投中科院 2 区肿瘤刊，给 3 个选题并打分。",
    ),
    help(
        "I will propose three topics under the data constraint, score novelty, feasibility, and journal fit, and name next week’s analysis.",
        "按数据约束出三个题目，评分创新、可行性和期刊拟合，并指定下一周该补的分析。",
    ),
);

const KNOWLEDGE: CapabilityIntro = intro(
    help(
        "Search a local WeKnora library of ingested papers or notes and cite excerpts.\nIf it is not connected, open the gear first, then say something as specific as: “Search my knowledge base for our notes on bile acids and fibrosis, and answer with quoted sentences”.",
        "要检索本机 WeKnora 里已入库的文献或笔记，并引用片段。\n未连接先点齿轮，第一句尽量写具体，例如：「在我的知识库里找我们组关于胆汁酸和纤维化的笔记，引用原句回答」。",
    ),
    help(
        "Lab notes live in WeKnora (or are not connected) and chat invents an answer anyway.\nA search returns no excerpt, but the model still writes a confident paragraph.\nYou cannot see which note file a sentence came from.\nSetup is lectured about instead of running a retrieval.",
        "组内笔记在 WeKnora 里（或根本没接通），对话却照样编答案。\n检索没有片段，模型仍写出很有把握的一段。\n看不到一句话出自哪份笔记。\n先被教育怎么配置，而不是先检索。",
    ),
    help(
        "Quoted excerpts from the local store | Answers cite titles or filenames from the configured knowledge base.\nHonest misses | If nothing matches, it says so instead of inventing a note.\nNot a chat API | Retrieval is a tool call; the reply is written from the excerpts.\nSetup stays in settings | This card searches; connecting WeKnora is a separate step.",
        "引用本地库里的原句 | 回答会引用已配置知识库里的标题或文件名。\n查不到就直说 | 没有命中就承认，不编一条笔记。\n不是聊天 API | 检索是工具调用，回答根据片段来写。\n配置留在设置里 | 这张卡负责查；接通 WeKnora 是另一步。",
    ),
    help(
        "If it is not connected, open the gear first\nThen click the card and ask a retrieval question\nStart with the actual task, for example: “Search my knowledge base for our notes on bile acids and fibrosis, and answer with quoted sentences.”",
        "未连接先点齿轮\n连上后点卡片，用问题检索\n第一句直接进入任务，可参考：「在我的知识库里找我们组关于胆汁酸和纤维化的笔记，引用原句回答。」",
    ),
    help(
        "Search my knowledge base for our notes on bile acids and fibrosis, and answer with quoted sentences.",
        "在我的知识库里找我们组关于胆汁酸和纤维化的笔记，引用原句回答。",
    ),
    help(
        "I will search WeKnora, quote excerpts, and list the source entries.",
        "检索 WeKnora 并引用片段作答，列出来源条目。",
    ),
);

const CHANNELS: CapabilityIntro = intro(
    help(
        "Drive the same desktop research session from Feishu or WeChat when you are not at the computer.\nClick the card to open channel settings and finish Feishu or WeChat setup, then say something as specific as: “After Feishu is connected, ping me there when a long run finishes”.",
        "人在手机或飞书里，也要驱动同一桌面科研会话。\n点卡片打开通道设置，按指引完成飞书或微信接入，第一句尽量写具体，例如：「接好飞书后，长任务结束请在飞书提醒我」。\n第一句可以写成：「接好飞书后，长任务结束请在飞书提醒我。」",
    ),
    help(
        "A long Run only moves if you are sitting at the desktop window.\nFeishu or WeChat is where you actually live, but the session stays trapped on the Mac.\nFinish notices never leave the machine, so overnight jobs die silently.\nChannel setup is unfinished and messages never enter the same session.",
        "长任务只有人坐在桌面窗口前才会往前走。\n人在飞书或微信，会话却锁在 Mac 上。\n完成通知出不了这台机器，过夜任务会默默死掉。\n通道没接完，消息进不了同一会话。",
    ),
    help(
        "One session from connected chat apps | After setup, Feishu or WeChat talks to the same desktop research session.\nProgress pings on the go | A finished Run can notify you where you already are.\nSettings first, then talk | The card opens channel setup; chat starts after the pipe exists.\nSame project state | Mobile pings do not fork a second, empty conversation.",
        "通道设置接通后，消息进同一会话 | 接好后，飞书或微信驱动的是同一桌面科研会话。\n适合催进度、收通知 | Run 结束可以通知到你人在的地方。\n先设置再对话 | 卡片打开通道设置；管道通了再聊。\n项目状态不分裂 | 手机提醒不会分出第二段空白对话。",
    ),
    help(
        "Click the card to open channel settings and finish Feishu or WeChat setup\nStart with the actual task, for example: “After Feishu is connected, ping me there when a long run finishes.”\nIf a file, ID, or hard limit is missing, add it in the next message instead of restarting with a feature lecture.",
        "点卡片打开通道设置，按指引完成飞书或微信接入\n第一句直接进入任务，可参考：「接好飞书后，长任务结束请在飞书提醒我。」\n若还缺文件、编号或硬限制，下一句直接补上，不必重开一轮功能介绍。",
    ),
    help(
        "After Feishu is connected, ping me there when a long run finishes.",
        "接好飞书之后，长任务结束请在飞书里提醒我一声。",
    ),
    help(
        "I will confirm the channel is connected and say that Run-complete notices go to the same session.",
        "确认通道已连接，并说明 Runs 完成通知会进同一会话。",
    ),
);

const BROWSER: CapabilityIntro = intro(
    help(
        "The site needs your logged-in Chrome — submission systems, campus libraries, or admin consoles that a headless script cannot enter.\nInstall the companion extension first, then say something as specific as: “Use my logged-in Chrome to open the journal submission page and check Figure upload rules. Do not submit”.",
        "网站要登录态，例如投稿系统、院内库、后台，无头脚本进不去。\n先装扩展，第一句尽量写具体，例如：「用我已登录的 Chrome 打开期刊投稿页，检查 Figure 上传格式要求，不要提交」。",
    ),
    help(
        "A submission site or campus library needs your logged-in Chrome; a headless script cannot enter.\nFigure-upload rules only exist behind that login.\nYou need to watch the page, not let a bot click Submit.\nThe companion extension is not installed, so the card cannot drive Chrome.",
        "投稿系统或院内库要你已登录的 Chrome，无头脚本进不去。\nFigure 上传规则只活在登录墙后面。\n你得看着页面走，不能让机器人点提交。\n配套扩展没装，卡片驱动不了 Chrome。",
    ),
    help(
        "A companion extension drives real local Chrome | Steps run in the browser you already logged into.\nYou can watch the page | The session is visible, not hidden in a headless window.\nNo accidental submit | Collection tasks can be bounded so Submit is never clicked.\nLogin stays yours | Cookies never need to be exported into the agent.",
        "通过配套扩展操作本机 Chrome | 步骤跑在你已经登录的浏览器里。\n你看着页面走 | 过程看得见，不是藏在无头窗口。\n避免误点提交 | 采集任务可以限定为绝不点 Submit。\n登录态仍是你的 | 不必把 Cookie 导出给助手。",
    ),
    help(
        "Install the companion extension first\nClick the card and name the site and the steps\nStart with the actual task, for example: “Use my logged-in Chrome to open the journal submission page and check Figure upload rules. Do not submit.”",
        "先装扩展\n再点卡片说明要打开的站点与步骤\n第一句直接进入任务，可参考：「用我已登录的 Chrome 打开期刊投稿页，检查 Figure 上传格式要求，不要提交。」",
    ),
    help(
        "Use my logged-in Chrome to open the journal submission page and check Figure upload rules. Do not submit.",
        "用我已登录的 Chrome 打开期刊投稿页，检查 Figure 上传格式要求，不要提交。",
    ),
    help(
        "I will drive the current Chrome through the extension, collect the format rules, and not click submit.",
        "经扩展控制当前 Chrome，只采集格式要求并回报，不点提交。",
    ),
);

const PLAYWRIGHT: CapabilityIntro = intro(
    help(
        "Script a headless Chromium, Firefox, or WebKit browser for tests, screenshots, or public-page scraping — no login session required.\nClick the card and give a URL, assertions, or selectors to capture, then say something as specific as: “Open our project reader.html with Playwright, screenshot the first screen, and check that both language columns exist”.",
        "要脚本化测试、截图、抓取公开页，不需要你的登录态。\n点卡片，说明 URL、断言或要截的选择器，第一句尽量写具体，例如：「用 Playwright 打开我们项目里的 reader.html，截第一屏并检查中英两栏都在」。",
    ),
    help(
        "A public page or local report needs a repeatable screenshot and assertion.\nManual clicking cannot be rerun after a CSS change.\nYou do not need a login: headless Chromium, Firefox, or WebKit is enough.\nQA of reader.html never becomes a script, so regressions surprise you later.",
        "公开页或本地报告需要可重复的截图和断言。\nCSS 一改，手工点过的步骤无法重跑。\n不需要登录态：无头 Chromium / Firefox / WebKit 就够。\nreader.html 的验收从未变成脚本，回归会在以后突然出现。",
    ),
    help(
        "Repeatable scripts | Open, assert, and screenshot become a path you can run again.\nMultiple browsers | Chromium, Firefox, or WebKit can be chosen for the same check.\nPublic-page QA | Logged-out pages and local HTML reports are the fit; logged-in sites belong on Browser.\nEvidence you can attach | A screenshot plus a pass/fail is more useful than “it looked fine”.",
        "可重复脚本 | 打开、断言、截图变成能再跑的路径。\n多浏览器 | 同一检查可选 Chromium、Firefox 或 WebKit。\n适合公开页 QA | 未登录页和本地 HTML 报告是它的场；要登录的站点用浏览器卡。\n能附上的证据 | 截图加通过/失败，比「看起来没问题」有用。",
    ),
    help(
        "Click the card and give a URL, assertions, or selectors to capture\nStart with the actual task, for example: “Open our project reader.html with Playwright, screenshot the first screen, and check that both language columns exist.”\nIf a file, ID, or hard limit is missing, add it in the next message instead of restarting with a feature lecture.",
        "点卡片，说明 URL、断言或要截的选择器\n第一句直接进入任务，可参考：「用 Playwright 打开我们项目里的 reader.html，截第一屏并检查中英两栏都在。」\n若还缺文件、编号或硬限制，下一句直接补上，不必重开一轮功能介绍。",
    ),
    help(
        "Open our project reader.html with Playwright, screenshot the first screen, and check that both language columns exist.",
        "用 Playwright 打开我们项目里的 reader.html，截第一屏并检查中英两栏都在。",
    ),
    help(
        "I will open the local report headlessly, screenshot it, and report whether both columns are present.",
        "无头打开本地报告，截图并回报两栏是否存在。",
    ),
);

const PLUGINS: CapabilityIntro = intro(
    help(
        "The bundled skills are not enough; you need extra Skill plus MCP packs the workbench does not ship.\nClick the card to open plugin settings, then return to chat after install, then say something as specific as: “Install a plugin that reads that instrument’s raw files, then parse data/raw/”.",
        "工作台默认技能不够，要加第三方 Skill + MCP 包。\n点卡片打开插件设置，安装后再回对话调用，第一句尽量写具体，例如：「安装能读某类仪器原始文件的插件，装好后帮我解析 data/raw/」。",
    ),
    help(
        "A vendor instrument format is not in the bundled skills.\nInstalled tools never enter the agent loop, so chat cannot call them.\nPlugin settings and the conversation stay disconnected.\nYou cannot parse data/raw/ until a third-party Skill plus MCP pack is installed.",
        "某类仪器原始格式不在默认技能里。\n装了的工具进不了助手循环，对话调用不到。\n插件设置和对话是断开的。\n不装第三方 Skill + MCP 包，就解析不了 data/raw/。",
    ),
    help(
        "Install and manage packs in plugin settings | Extra Skill plus MCP packs are added in one place.\nNew tools appear in chat | After install, the agent loop can call the new parser.\nVendor formats become reachable | Instrument raw files are no longer a dead folder.\nSettings, then a real task | The card opens the installer; the next message can point at data/raw/.",
        "在插件设置安装和管理扩展 | 额外的 Skill + MCP 包在同一处添加。\n新工具出现在对话里 | 装好后，助手循环就能调用新解析器。\n仪器格式变得可及 | 原始文件不再是死文件夹。\n先设置再做真任务 | 卡片打开安装；下一句就可以指向 data/raw/。",
    ),
    help(
        "Click the card to open plugin settings, then return to chat after install\nStart with the actual task, for example: “Install a plugin that reads that instrument’s raw files, then parse data/raw/.”\nIf a file, ID, or hard limit is missing, add it in the next message instead of restarting with a feature lecture.",
        "点卡片打开插件设置，安装后再回对话调用\n第一句直接进入任务，可参考：「安装能读某类仪器原始文件的插件，装好后帮我解析 data/raw/。」\n若还缺文件、编号或硬限制，下一句直接补上，不必重开一轮功能介绍。",
    ),
    help(
        "Install a plugin that reads that instrument’s raw files, then parse data/raw/.",
        "安装能读某类仪器原始文件的插件，装好后帮我解析 data/raw/。",
    ),
    help(
        "After the pack is installed in plugin settings I will parse that directory with the new tool and report its structure.",
        "在插件设置完成安装后，用新工具解析该目录并回报结构。",
    ),
);

const SKILLS: CapabilityIntro = intro(
    help(
        "You already have bundled skills, but you need to see what is installed, turn one off, or add a project Skill before chat can load it.\nClick the card to open skill settings, then return to chat and say something as specific as: “Enable academic-paper, then outline a PD-1 review for Nature Reviews Immunology”.",
        "默认技能已经在，但你要先看装了哪些、关掉不用的，或给项目加一个技能，对话才能加载它。\n点卡片打开技能设置，再回对话，第一句尽量写具体，例如：「打开 academic-paper，按 Nature Reviews Immunology 给一篇 PD-1 综述列大纲」。",
    ),
    help(
        "You cannot tell which Skills the agent is allowed to load.\nA noisy or unused Skill stays on and crowds the next turn.\nA project-only Skill never appears until you add it in settings.\nChat cannot follow a SKILL.md that is disabled or missing.",
        "看不清助手现在能加载哪些技能。\n用不到的技能一直开着，下一轮会被挤乱。\n项目专用技能不进设置就出不来。\n技能关掉或缺失时，对话跟不上 SKILL.md。",
    ),
    help(
        "One place to browse and toggle Skills | Installed Skills are listed with enable switches in settings.\nAdd a file, folder, or ZIP | Project or extra Skills enter the catalog from this settings page.\nChat loads only what you left on | Disabled Skills stay out of the next agent turn.\nSettings, then a real writing task | The card opens the catalog; the next message can name a Skill.",
        "在同一处浏览和开关技能 | 已装技能在设置页里带开关列出来。\n可添加文件、文件夹或 ZIP | 项目技能或额外技能从这页设置进入目录。\n对话只加载你还开着的 | 关掉的技能不会挤进下一轮对话。\n先设置再做真任务 | 卡片打开技能目录，下一句就可以点名一个技能。",
    ),
    help(
        "Click the card to open skill settings\nBrowse or add the Skill you need, then enable it\nReturn to chat and start with the actual task, for example: “Enable academic-paper, then outline a PD-1 review for Nature Reviews Immunology.”",
        "点卡片打开技能设置\n浏览或添加要用的技能，并打开开关\n回到对话后直接进入任务，可参考：「打开 academic-paper，按 Nature Reviews Immunology 给一篇 PD-1 综述列大纲。」",
    ),
    help(
        "Enable academic-paper, then outline a PD-1 review for Nature Reviews Immunology.",
        "打开 academic-paper，按 Nature Reviews Immunology 给一篇 PD-1 综述列大纲。",
    ),
    help(
        "I will confirm academic-paper is enabled, then give a one-page outline aimed at that journal.",
        "先确认 academic-paper 已打开，再按该期刊给一页大纲。",
    ),
);
