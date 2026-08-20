#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
search_engine.py — 学术检索引擎核心实现（能力层落地脚本）

实现 SKILL.md 六阶段引擎的**可执行部分**：
  Phase2 源路由 / Phase3 跨库查询构建 / Phase4 归一化+三级去重 / Phase5 排序 / Phase6 JSON 矩阵
纯标准库（urllib）发请求，无第三方依赖；内置礼貌限速（每源最小间隔）。
中文库（CNKI/万方/百度学术）不在本脚本内——按 SKILL.md 用 web_search 合规检索后，
用 normalize_record() 把结果并入同一矩阵即可。

运行：
  python search_engine.py "deep learning transformer" --n 20 --src openalex crossref \
         --out matrix.json --mode relevance
无参数运行触发离线自检（归一化 + 三级去重逻辑验证，不发网络请求）。
"""
import argparse
import json
import re
import sys
import time
import urllib.parse
import urllib.request
import xml.etree.ElementTree as ET
from difflib import SequenceMatcher

UA = "academic-search-pro/1.0 (mailto:researcher@example.com)"
_RATE = {}  # 源 -> 上次请求时间

def _get(url, timeout=20, accept="application/json"):
    host = urllib.parse.urlparse(url).netloc
    now = time.time()
    gap = 1.0 if "arxiv" in host else 0.34  # 粗略礼貌限速
    if host in _RATE and now - _RATE[host] < gap:
        time.sleep(gap - (now - _RATE[host]))
    _RATE[host] = time.time()
    req = urllib.request.Request(url, headers={"User-Agent": UA, "Accept": accept})
    with urllib.request.urlopen(req, timeout=timeout) as r:
        data = r.read().decode("utf-8", "replace")
    if accept.startswith("application/json"):
        return json.loads(data)
    return data

# ---------------------------------------------------------------------------
# 1. 归一化
# ---------------------------------------------------------------------------
def norm_title(s):
    if not s:
        return ""
    s = s.lower()
    s = re.sub(r"[^a-z0-9\u4e00-\u9fff]+", " ", s)
    return s.strip()

def _first_author(authors):
    if authors and isinstance(authors, list) and authors[0]:
        a = authors[0]
        if isinstance(a, dict):
            return (a.get("family") or a.get("name") or a.get("display_name") or "").split()[-1]
        return str(a).split()[-1]
    return ""

def normalize_record(raw, source):
    """把各库原始记录统一为 SKILL.md 4.2 的归一化结构（子集，按需扩展）。"""
    r = {
        "title": (raw.get("title") or "").strip(),
        "authors": raw.get("authors") or [],
        "year": raw.get("year"),
        "doi": (raw.get("doi") or "").lower().replace("https://doi.org/", "").strip(),
        "journal": raw.get("journal") or "",
        "citation_count": raw.get("citation_count") or 0,
        "url": raw.get("url") or "",
        "is_open_access": bool(raw.get("is_open_access")),
        "source_type": raw.get("source_type") or "other",
        "sources": [source],
    }
    r["norm_title"] = norm_title(r["title"])
    r["first_author"] = _first_author(r["authors"])
    return r

# ---------------------------------------------------------------------------
# 2. 各库查询构建 + 解析
# ---------------------------------------------------------------------------
def fetch_openalex(query, n=20):
    url = "https://api.openalex.org/works?search=%s&per-page=%d&mailto=researcher@example.com" % (
        urllib.parse.quote(query), n)
    try:
        d = _get(url)
    except Exception as e:
        return [], f"openalex:{e}"
    out = []
    for w in d.get("results", []):
        auth = [a["author"].get("display_name", "") for a in w.get("authorships", [])]
        loc = w.get("primary_location") or {}
        src = (loc.get("source") or {})
        out.append(normalize_record({
            "title": w.get("display_name", ""), "authors": auth,
            "year": w.get("publication_year"), "doi": w.get("doi", ""),
            "journal": src.get("display_name", ""), "citation_count": w.get("cited_by_count", 0),
            "url": (loc.get("landing_page_url") or w.get("id", "")),
            "is_open_access": bool((w.get("open_access") or {}).get("is_oa")),
            "source_type": w.get("type", "other"),
        }, "openalex"))
    return out, None

def fetch_crossref(query, n=20):
    url = "https://api.crossref.org/works?query=%s&rows=%d" % (urllib.parse.quote(query), n)
    try:
        d = _get(url)
    except Exception as e:
        return [], f"crossref:{e}"
    out = []
    for it in d.get("message", {}).get("items", []):
        auth = []
        for a in it.get("author", []):
            nm = " ".join(filter(None, [a.get("given", ""), a.get("family", "")])).strip()
            if nm:
                auth.append(nm)
        yr = None
        dp = (it.get("issued") or {}).get("date-parts", [[]])
        if dp and dp[0]:
            yr = dp[0][0]
        out.append(normalize_record({
            "title": " ".join(it.get("title", [])) if it.get("title") else "",
            "authors": auth, "year": yr, "doi": it.get("DOI", ""),
            "journal": " ".join(it.get("container-title", [])) if it.get("container-title") else "",
            "citation_count": it.get("is-referenced-by-count", 0),
            "url": (it.get("URL") or ""),
            "is_open_access": "license" in it or bool(it.get("oa_status")),
            "source_type": it.get("type", "other"),
        }, "crossref"))
    return out, None

def fetch_semantic_scholar(query, n=20, api_key=""):
    url = "https://api.semanticscholar.org/graph/v1/paper/search?query=%s&limit=%d" % (
        urllib.parse.quote(query), n)
    if api_key:
        url += "&fields=title,authors,year,citationCount,doi,venue,externalIds"
    try:
        d = _get(url)
    except Exception as e:
        return [], f"semantic:{e}"
    out = []
    for p in d.get("data", []):
        auth = [a.get("name", "") for a in p.get("authors", [])]
        out.append(normalize_record({
            "title": p.get("title", ""), "authors": auth, "year": p.get("year"),
            "doi": (p.get("externalIds") or {}).get("DOI", ""), "journal": p.get("venue", ""),
            "citation_count": p.get("citationCount", 0),
            "url": "https://www.semanticscholar.org/paper/%s" % p.get("paperId", ""),
            "is_open_access": False, "source_type": "other",
        }, "semantic_scholar"))
    return out, None

def fetch_arxiv(query, n=20):
    url = "http://export.arxiv.org/api/query?search_query=all:%s&max_results=%d" % (
        urllib.parse.quote(query), n)
    try:
        xml = _get(url, accept="application/atom+xml")
    except Exception as e:
        return [], f"arxiv:{e}"
    out = []
    try:
        root = ET.fromstring(xml)
    except ET.ParseError as e:
        return [], f"arxiv.parse:{e}"
    ns = {"a": "http://www.w3.org/2005/Atom"}
    for e in root.findall("a:entry", ns):
        title = " ".join((e.findtext("a:title", default="", namespaces=ns)).split())
        auth = [a.findtext("a:name", default="", namespaces=ns)
                for a in e.findall("a:author", ns)]
        yr = (e.findtext("a:published", default="", namespaces=ns) or "")[:4]
        out.append(normalize_record({
            "title": title, "authors": auth, "year": int(yr) if yr.isdigit() else None,
            "doi": "", "journal": "arXiv", "citation_count": 0,
            "url": e.findtext("a:id", default="", namespaces=ns),
            "is_open_access": True, "source_type": "preprint",
        }, "arxiv"))
    return out, None

def fetch_pubmed(query, n=20):
    try:
        s = _get("https://eutils.ncbi.nlm.nih.gov/entrez/eutils/esearch.fcgi?db=pubmed&retmode=json&retmax=%d&term=%s"
                 % (n, urllib.parse.quote(query)))
        ids = s.get("esearchresult", {}).get("idlist", [])
        if not ids:
            return [], None
        sumr = _get("https://eutils.ncbi.nlm.nih.gov/entrez/eutils/esummary.fcgi?db=pubmed&retmode=json&id=%s"
                    % ",".join(ids))
    except Exception as e:
        return [], f"pubmed:{e}"
    out = []
    for pid, u in sumr.get("result", {}).items():
        if pid == "uids":
            continue
        auth = [a.get("name", "") for a in u.get("authors", [])]
        out.append(normalize_record({
            "title": u.get("title", ""), "authors": auth, "year": u.get("pubdate", "")[:4] or None,
            "doi": (u.get("articleids") and next((i["value"] for i in u["articleids"]
                                                  if i["idtype"] == "doi"), "")) or "",
            "journal": u.get("source", ""), "citation_count": u.get("citedbycount", 0),
            "url": "https://pubmed.ncbi.nlm.nih.gov/%s" % pid,
            "is_open_access": False, "source_type": "journal-article",
        }, "pubmed"))
    return out, None

def fetch_doaj(query, n=20):
    url = "https://doaj.org/api/search/articles/%s?pageSize=%d" % (urllib.parse.quote(query), n)
    try:
        d = _get(url)
    except Exception as e:
        return [], f"doaj:{e}"
    out = []
    for r in d.get("results", []):
        b = r.get("bibjson", {})
        auth = [a.get("name", "") for a in b.get("author", [])]
        yr = (b.get("year") or {}).get("min") if isinstance(b.get("year"), dict) else b.get("year")
        out.append(normalize_record({
            "title": " ".join(b.get("title", [])) if isinstance(b.get("title"), list) else b.get("title", ""),
            "authors": auth, "year": int(yr) if str(yr).isdigit() else None,
            "doi": (b.get("identifier") and next((i["id"] for i in b["identifier"]
                                                   if i.get("type") == "doi"), "")) or "",
            "journal": (b.get("journal", {}) or {}).get("title", "") if isinstance(b.get("journal"), dict) else "",
            "citation_count": 0, "url": r.get("fulltext", ""),
            "is_open_access": True, "source_type": "journal-article",
        }, "doaj"))
    return out, None

FETCHERS = {
    "openalex": fetch_openalex, "crossref": fetch_crossref,
    "semantic_scholar": fetch_semantic_scholar, "arxiv": fetch_arxiv,
    "pubmed": fetch_pubmed, "doaj": fetch_doaj,
}

# ---------------------------------------------------------------------------
# 3. 三级去重
# ---------------------------------------------------------------------------
def dedup(records):
    """Level1 DOI → Level2 标题相似≥0.90 → Level3 标题+一作+年份。返回 (去重后, 统计)。"""
    by_doi, rest, seen = {}, [], set()
    for r in records:
        if r["doi"]:
            if r["doi"] in by_doi:
                if "doi" not in by_doi[r["doi"]]["sources"]:
                    by_doi[r["doi"]]["sources"].append(r["sources"][0])
            else:
                by_doi[r["doi"]] = dict(r)
        else:
            rest.append(r)
    kept = list(by_doi.values())
    merged_l2 = 0
    for r in rest:
        hit = None
        for k in kept:
            if k["doi"]:
                continue
            if SequenceMatcher(None, r["norm_title"], k["norm_title"]).ratio() >= 0.90:
                hit = k; break
            if (r["norm_title"] and r["norm_title"] == k["norm_title"]
                    and r["first_author"] == k["first_author"] and r["year"] == k["year"]):
                hit = k; break
        if hit is None:
            kept.append(dict(r))
        else:
            if r["sources"][0] not in hit["sources"]:
                hit["sources"].append(r["sources"][0])
            merged_l2 += 1
    stats = {"input": len(records), "output": len(kept), "merged": len(records) - len(kept)}
    return kept, stats

# ---------------------------------------------------------------------------
# 4. 源路由 + 排序
# ---------------------------------------------------------------------------
def route_sources(intent="general", lang="any", domain="general"):
    sel = []
    if lang in ("zh", "cn", "中文"):
        sel += ["openalex", "crossref"]  # 补 DOI 便于归一化；中文库走 web_search
    if domain == "biomed":
        sel += ["pubmed", "openalex", "semantic_scholar", "doaj"]
    elif domain in ("cs", "physics", "math"):
        sel += ["arxiv", "semantic_scholar", "openalex", "crossref"]
    elif intent == "citation":
        sel += ["semantic_scholar", "openalex"]
    elif intent == "oa":
        sel += ["doaj", "core", "arxiv"]
    else:
        sel += ["openalex", "crossref", "semantic_scholar"]
    return list(dict.fromkeys(sel))

def rank(records, mode="relevance", current_year=2026):
    if mode == "citations":
        return sorted(records, key=lambda r: r["citation_count"], reverse=True)
    if mode == "newest":
        return sorted(records, key=lambda r: r["year"] or 0, reverse=True)
    if mode == "oldest":
        return sorted(records, key=lambda r: r["year"] or 9999)
    # relevance = 0.6*log(1+cite) + 0.4*recency
    def score(r):
        import math
        rec = 1 / (1 + 0.1 * max(0, (current_year - (r["year"] or current_year))) ** 2)
        return 0.6 * math.log1p(r["citation_count"]) + 0.4 * rec
    return sorted(records, key=score, reverse=True)

# ---------------------------------------------------------------------------
# 5. 编排
# ---------------------------------------------------------------------------
def search(query, sources=None, n=20, mode="relevance", api_key="", current_year=2026):
    if sources is None:
        sources = ["openalex", "crossref", "semantic_scholar"]
    all_rec, errors = [], {}
    for s in sources:
        if s not in FETCHERS:
            continue
        recs, err = FETCHERS[s](query, n, api_key) if s == "semantic_scholar" else FETCHERS[s](query, n)
        if err:
            errors[s] = err
            continue
        all_rec += recs
    deduped, stats = dedup(all_rec)
    ranked = rank(deduped, mode, current_year)
    matrix = {
        "_meta": {
            "origin": "academic-search-pro/search_engine.py",
            "format": "literature_matrix", "cascade_ready": True,
            "next_candidates": ["literature-mining"],
            "search_query": query, "sources_used": sources,
            "total_results": len(ranked), "dedup_stats": stats,
            "retrieved_at": time.strftime("%Y-%m-%dT%H:%M:%S%z"),
        },
        "papers": [{
            "title": r["title"], "authors": r["authors"], "year": r["year"],
            "doi": r["doi"], "journal": r["journal"], "citation_count": r["citation_count"],
            "url": r["url"], "is_open_access": r["is_open_access"],
            "sources": r["sources"],
        } for r in ranked],
        "_errors": errors,
    }
    return matrix

# ---------------------------------------------------------------------------
# 6. CLI
# ---------------------------------------------------------------------------
def main():
    ap = argparse.ArgumentParser(description="学术检索引擎核心")
    ap.add_argument("query")
    ap.add_argument("--n", type=int, default=20)
    ap.add_argument("--src", nargs="+",
                    default=["openalex", "crossref", "semantic_scholar"])
    ap.add_argument("--mode", default="relevance")
    ap.add_argument("--key", default="")
    ap.add_argument("--out", help="输出 JSON 矩阵路径")
    args = ap.parse_args()
    m = search(args.query, args.src, args.n, args.mode, args.key)
    txt = json.dumps(m, ensure_ascii=False, indent=2)
    if args.out:
        with open(args.out, "w", encoding="utf-8") as f:
            f.write(txt)
        print(f"[ok] 矩阵 -> {args.out}  (去重前{args.src}: {m['_meta']['dedup_stats']})")
    else:
        print(txt)

if __name__ == "__main__":
    if len(sys.argv) == 1:
        # 离线自检：归一化 + 三级去重
        a = normalize_record({"title": "Attention Is All You Need", "authors": ["Vaswani et al."],
                              "year": 2017, "doi": "10.48550/arXiv.1706.03762"}, "openalex")
        b = normalize_record({"title": "attention is all you need", "authors": ["Vaswani"],
                              "year": 2017, "doi": "10.48550/ARXIV.1706.03762"}, "crossref")
        c = normalize_record({"title": "Deep Learning", "authors": ["LeCun"], "year": 2015}, "arxiv")
        d, st = dedup([a, b, c])
        print("输入3条，去重后=", len(d), "统计=", st)
        assert len(d) == 2, "三级去重失败"
        print("源路由(general)=", route_sources())
        print("源路由(biomed)=", route_sources(domain="biomed"))
        print("模块能力可用。")
    else:
        main()
