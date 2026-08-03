"""Export ESR1 sessions to bundled seed manifests with redaction."""

from __future__ import annotations

import json
import re
import sqlite3
import tarfile
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
SEED = ROOT / "seed"
SRC = Path(r"D:\Wisp-Science\ESR1_ws")
DB = Path(
    r"C:\Users\xuzhougeng\AppData\Roaming\science.wisp-science\wisp-science\wisp.sqlite"
)
MAX_TOOL_TEXT = 6000
MAX_REASONING = 4000

# frame_id -> (manifest stem without .json, demo root id)
SESSIONS = [
    (
        "0a662fc6-da20-4a60-bda7-1a15c6d2fe5a",
        "manifest_esr1_rnaseq",
        "demo-esr1-rnaseq",
    ),
    (
        "5602d6f7-80d1-447f-9ef3-56e1eb4d99eb",
        "manifest_esr1_datasets",
        "demo-esr1-datasets",
    ),
    (
        "3670bb53-ed32-44e5-9ee3-e8ca741ed8ec",
        "manifest_esr1_samples",
        "demo-esr1-samples",
    ),
]

REDACT = [
    (re.compile(r"English reply\.\s*", re.I), ""),
    (re.compile(r"guotosky", re.I), "remote-host"),
    (re.compile(r"ssh:remote-host", re.I), "ssh:remote-host"),
    (re.compile(r"10\.10\.10\.\d+(:\d+)?"), "configured-proxy"),
    (re.compile(r"http://10\.10\.10\.\d+(:\d+)?"), "configured-proxy"),
    (re.compile(r"D:\\\\ESR1_project", re.I), "data"),
    (re.compile(r"D:/ESR1_project", re.I), "data"),
    (re.compile(r"D:\\Wisp-Science\\ESR1_ws", re.I), "."),
    (re.compile(r"D:/Wisp-Science/ESR1_ws", re.I), "."),
    (re.compile(r"~/GSE153250_ESR1", re.I), "~/workspace/GSE153250"),
    (re.compile(r"miniconda3", re.I), "conda-tools"),
    (re.compile(r"~/bin/(\w+)", re.I), r"~/tools/\1"),
    (re.compile(r"~/.local/bin/(\w+)", re.I), r"~/tools/\1"),
    (re.compile(r"/home/data/gz0548/", re.I), "~/"),
    (re.compile(r"/home/data/gz0548", re.I), "~"),
]


def redact(s: str) -> str:
    if not s:
        return s
    out = s
    for pat, repl in REDACT:
        out = pat.sub(repl, out)
    return out


def truncate(s: str, limit: int) -> str:
    if len(s) <= limit:
        return s
    return s[: limit - 80] + f"\n\n… [truncated, {len(s) - limit + 80} chars omitted]"


def unwrap_text(raw: str) -> str:
    """Unwrap JSON-encoded string layers from SQLite message content."""
    if not raw:
        return ""
    s = raw.strip()
    for _ in range(4):
        if not s:
            return s
        if s[0] in '"[{':
            try:
                v = json.loads(s)
            except json.JSONDecodeError:
                break
            if isinstance(v, str):
                s = v
                continue
            if isinstance(v, dict):
                if "text" in v or "content" in v:
                    text = v.get("text") or v.get("content") or ""
                    if isinstance(text, list):
                        parts = []
                        for block in text:
                            if isinstance(block, dict) and block.get("type") == "text":
                                parts.append(block.get("text", ""))
                        return "".join(parts)
                    if isinstance(text, str):
                        return text
                return json.dumps(v, ensure_ascii=False)
            break
        break
    return s


def parse_message_content(raw: str) -> str:
    return unwrap_text(raw)


def is_run_json(text: str) -> bool:
    if not text.startswith("{"):
        return False
    try:
        v = json.loads(text)
    except json.JSONDecodeError:
        return False
    return isinstance(v, dict) and ("run_id" in v or "id" in v) and "status" in v


def messages_to_ui_items(rows: list[sqlite3.Row]) -> list[dict]:
    tool_inputs: dict[str, str] = {}
    for row in rows:
        if row["role"] != "assistant":
            continue
        tc = row["tool_calls"]
        if not tc:
            continue
        try:
            calls = json.loads(tc)
        except json.JSONDecodeError:
            continue
        for call in calls if isinstance(calls, list) else []:
            fn = call.get("function") or {}
            name = fn.get("name") or ""
            cid = call.get("id") or ""
            args_raw = fn.get("arguments") or "{}"
            try:
                args = json.loads(args_raw) if isinstance(args_raw, str) else args_raw
            except json.JSONDecodeError:
                continue
            val = None
            if name in ("python", "r"):
                val = args.get("code")
            elif name == "shell":
                val = args.get("cmd")
            elif name in ("monitor_run", "wisp_monitor_run"):
                val = args.get("run_id")
            if val and cid:
                tool_inputs[cid] = val

    items: list[dict] = []
    for row in rows:
        role = row["role"]
        text = parse_message_content(row["content"] or "")
        tool_name = row["tool_name"]
        tool_call_id = row["tool_call_id"]

        if role == "system":
            continue

        if role == "user":
            t = text.strip()
            if not t:
                continue
            items.append(
                {
                    "role": "user",
                    "text": t,
                    "tool_name": None,
                    "ok": None,
                    "input": None,
                    "model_name": None,
                    "resources": [],
                }
            )
            continue

        if role == "assistant":
            reasoning_raw = row["reasoning"]
            if reasoning_raw and str(reasoning_raw).strip():
                rtext = unwrap_text(str(reasoning_raw))
                if rtext.strip():
                    items.append(
                        {
                            "role": "reasoning",
                            "text": rtext,
                            "tool_name": None,
                            "ok": None,
                            "input": None,
                            "model_name": None,
                            "resources": [],
                        }
                    )
            if text.strip():
                items.append(
                    {
                        "role": "assistant",
                        "text": text,
                        "tool_name": None,
                        "ok": None,
                        "input": None,
                        "model_name": row["model_name"],
                        "resources": [],
                    }
                )
            continue

        if role == "tool":
            if tool_name == "attempt_completion":
                if text.strip():
                    items.append(
                        {
                            "role": "assistant",
                            "text": text,
                            "tool_name": None,
                            "ok": None,
                            "input": None,
                            "model_name": row["model_name"],
                            "resources": [],
                        }
                    )
                continue
            if tool_name in ("propose_plan", "update_plan", "Plan"):
                items.append(
                    {
                        "role": "plan",
                        "text": text,
                        "tool_name": None,
                        "ok": None,
                        "input": None,
                        "model_name": None,
                        "resources": [],
                    }
                )
                continue

            inp = tool_call_id and tool_inputs.get(tool_call_id)
            items.append(
                {
                    "role": "tool",
                    "text": text,
                    "tool_name": tool_name,
                    "ok": True,
                    "input": inp,
                    "model_name": None,
                    "resources": [],
                }
            )

    return items


def redact_item(item: dict) -> dict:
    role = item["role"]
    limit = MAX_REASONING if role == "reasoning" else MAX_TOOL_TEXT
    if role == "tool" and is_run_json(item.get("text") or ""):
        limit = 12000
    text = truncate(redact(item.get("text") or ""), limit)
    inp = item.get("input")
    if isinstance(inp, str):
        inp = truncate(redact(inp), 4000)
    out = dict(item)
    out["text"] = text
    out["input"] = inp
    return out


def derive_summary(items: list[dict]) -> tuple[str, str, str | None]:
    request = next((i["text"] for i in items if i["role"] == "user"), "")
    response = ""
    thinking = None
    for i in reversed(items):
        if i["role"] == "assistant" and i["text"].strip():
            response = i["text"]
            break
    for i in items:
        if i["role"] == "reasoning" and i["text"].strip():
            thinking = i["text"]
            break
    return request, response, thinking


def export_session(
    con: sqlite3.Connection, frame_id: str, manifest_id: str, demo_id: str
) -> Path:
    rows = list(
        con.execute(
            """
            SELECT seq, role, content, tool_calls, tool_call_id, tool_name, reasoning, model_name
            FROM messages
            WHERE frame_id=?
            ORDER BY seq
            """,
            (frame_id,),
        )
    )
    if not rows:
        raise SystemExit(f"no messages for frame {frame_id}")

    items = [redact_item(i) for i in messages_to_ui_items(rows)]
    request, response, thinking = derive_summary(items)
    manifest = {
        "root_frame": {
            "id": demo_id,
            "parent_frame_id": None,
            "root_frame_id": demo_id,
            "agent_name": "WISP",
            "status": "completed",
            "input_data": {"request": request},
            "output_data": {
                "response": response,
                "thinking": thinking,
                "items": items,
            },
        }
    }
    path = SEED / f"{manifest_id}.json"
    path.write_text(json.dumps(manifest, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    print(
        "wrote",
        path.name,
        path.stat().st_size,
        "msgs",
        len(rows),
        "items",
        len(items),
        "tools",
        sum(1 for i in items if i["role"] == "tool"),
    )
    blob = path.read_text(encoding="utf-8")
    for bad in ("guotosky", "10.10.10.", "English reply"):
        assert bad.lower() not in blob.lower(), f"{path.name} still contains {bad}"
    return path


def build_rnaseq_assets() -> None:
    assets = [
        (
            "example_esr1_rnaseq/GSE153250_counts_matrix.tsv",
            SRC / "data" / "processed" / "GSE153250_counts_matrix.tsv",
        ),
        (
            "example_esr1_rnaseq/GSE153250_sample_groups.txt",
            SRC / "data" / "processed" / "GSE153250_sample_groups.txt",
        ),
        (
            "example_esr1_rnaseq/GSE153250_featureCounts_summary.txt",
            SRC / "data" / "processed" / "GSE153250_featureCounts_summary.txt",
        ),
    ]
    tar_path = SEED / "assets_esr1_rnaseq.tar.gz"
    with tarfile.open(tar_path, "w:gz", compresslevel=9) as tar:
        for arcname, src in assets:
            if not src.is_file():
                raise SystemExit(f"missing asset: {src}")
            tar.add(src, arcname=arcname)
    print("wrote", tar_path.name, tar_path.stat().st_size)


def main() -> None:
    if not DB.is_file():
        raise SystemExit(f"database not found: {DB}")

    SEED.mkdir(exist_ok=True)
    con = sqlite3.connect(f"file:{DB}?mode=ro", uri=True)
    con.row_factory = sqlite3.Row
    for frame_id, manifest_id, demo_id in SESSIONS:
        export_session(con, frame_id, manifest_id, demo_id)
    build_rnaseq_assets()
    total = sum(p.stat().st_size for p in SEED.iterdir() if p.is_file())
    print(f"seed total {total/1024:.1f} KiB")


if __name__ == "__main__":
    main()
