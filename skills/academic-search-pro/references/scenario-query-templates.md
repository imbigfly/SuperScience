# 分场景检索查询构建模板（能力层 · 把 S1–S8 落成查询串）

> SKILL.md 给了各库参数，本表补**每类场景具体怎么拼查询**：选哪些源、用什么布尔/字段限定、给可直接套的示例。
> 与 `scripts/search_engine.py` 配合：(query, sources) 即引擎入参。

| 场景 | 选源（route_sources 输出） | 查询构建要点 | 示例 |
|------|---------------------------|--------------|------|
| S1 快速关键词 | openalex,crossref,semantic_scholar | 原词直搜；多词用空格（AND） | `"封头 成形工艺 优化"` |
| S2 跨库综合 | 上 + 按需 arxiv/pubmed | 同 query 跑多源，去重合并 | 同上，加 `--src openalex crossref semantic_scholar arxiv` |
| S3 DOI/PMID 精确 | 单源（crossref/pubmed） | 走精确端点，不做关键词模糊 | DOI `10.48550/arXiv.1706.03762` → crossref |
| S4 作者/机构 | openalex,semantic_scholar | 作者名加机构过滤；同名加学科 | `author:"Wang Lei" AND aff:"Tsinghua"` |
| S5 中文文献 | openalex,crossref(补DOI) + web_search CNKI/万方/百度学术 | 中文词直搜；用 web_search 合规补中文源 | `web_search site:cnki.net "封头成形"` |
| S6 引文检索 | semantic_scholar,openalex | 先取源论文 id，再拉 references/cited_by | 用 S3 定位后取 `citations` 关系 |
| S7 主题综述(PICO) | openalex,crossref,pubmed,semantic | PICO 拆 4 组词，用 AND/OR 组合；记录检索式 | `(intervention: stent) AND (population: CAD) AND (outcome: restenosis)` |
| S8 专利检索 | 用 web_search + 各国专利库 API | 技术关键词 + 分类号；不提供法律意见 | `web_search "封头 旋压 专利 CN"` |

## 布尔与字段限定速查

- **AND / OR / NOT**：OpenAlex、Crossref、arXiv、PubMed(MeSH) 均支持；中文 web_search 用空格即 AND。
- **字段限定**：OpenAlex `title:`/`abstract:`；PubMed `MeSH:`；Crossref `query.bibliographic:`。
- **时间过滤**：OpenAlex `filter=from_publication_date:2020-01-01`；Crossref `filter=from-pub-date:2020-01-01`。
- **类型过滤**：Crossref `filter=type:journal-article`；OpenAlex `filter=type:article`。
- **OA 限定**：OpenAlex `filter=open_access:true`；CORE/DOAJ 天然 OA。

## 检索式可复现记录模板（S7 强制）

```
检索问题: {PICO}
检索式:   {库}::{完整 query 串}
数据库:   {list}
时间范围: {from}~{to}
检索日期: {ts}
命中/去重: {raw} / {dedup}
```
