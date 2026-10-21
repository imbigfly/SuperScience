#!/usr/bin/env python3
"""Sync allowlisted vendor skill packs into the bundled skills/ tree.

This mirrors `crates/superscience-skills/src/update.rs`:

* pick the same declared skill directories (repo root or `skills/`);
* rewrite the `wisp:` frontmatter key to `superscience:`;
* keep `nature-proposal-writer`'s directory-matching name;
* lift academic `shared/` to `skills/academic-shared` and rewrite references;
* copy OfficeCLI's root LICENSE/NOTICE into `skills/officecli`.

The runtime updater writes overlays under `~/.superscience/skills`; this script
updates the signed bundle instead. Intended for maintainers and the
`sync-vendor-skills` workflow, which runs on a GitHub runner so it does not
depend on a slow local link to codeload.github.com.
"""

from __future__ import annotations

import re
import shutil
import subprocess
import sys
import tempfile
from dataclasses import dataclass
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
SKILLS = ROOT / "skills"
UPDATE_RS = ROOT / "crates/superscience-skills/src/update.rs"

ACADEMIC_SHARED_SKILL_MD = """---
name: academic-shared
description: Internal shared contracts, protocols, and schemas for academic-paper, academic-paper-reviewer, academic-pipeline, and deep-research. Do not invoke it as a standalone user workflow. Load only the specific shared file requested by another academic skill.
---

# Academic Shared References

Use this package only as a dependency of another installed academic-research skill.

- Load the exact referenced file; do not preload the whole package.
- Treat `contracts/`, `references/`, and the root protocols as shared definitions, not standalone workflows.
- Return to the requesting skill for task logic, output format, and final QA.

Consuming skills reference files as `../academic-shared/...`.
"""


@dataclass(frozen=True)
class Pack:
    id: str
    repo: str
    ref: str
    ref_kind: str  # "tag" or "commit"
    vendor_dir: str
    skills: tuple[str, ...]
    source_lines: tuple[str, ...]
    short_old: str
    short_new: str
    full_old: str = ""
    full_new: str = ""
    old_date: str = ""
    new_date: str = ""


PACKS: tuple[Pack, ...] = (
    Pack(
        id="nature-skills",
        repo="https://github.com/Yuan1z0825/nature-skills.git",
        ref="2a163b95255c608609ff815f3bbc6266591598e2",
        ref_kind="commit",
        vendor_dir="nature-skills",
        skills=(
            "nature-academic-search",
            "nature-citation",
            "nature-data",
            "nature-downloader",
            "nature-experiment-log",
            "nature-figure",
            "nature-image2ppt",
            "nature-literature-pipeline",
            "nature-paper-card",
            "nature-paper-to-patent",
            "nature-paper2ppt",
            "nature-polishing",
            "nature-proposal-writer",
            "nature-reader",
            "nature-ref-verifier",
            "nature-response",
            "nature-reviewer",
            "nature-shared",
            "nature-statistics",
            "nature-writing",
        ),
        source_lines=(),
        short_old="2661b531",
        short_new="2a163b95",
        full_old="2661b53150b537534159a2d4c6e35b1c9056f1e4",
        full_new="2a163b95255c608609ff815f3bbc6266591598e2",
        old_date="2026-09-13",
        new_date="2026-10-20",
    ),
    Pack(
        id="academic-research-skills",
        repo="https://github.com/Imbad0202/academic-research-skills.git",
        ref="v3.23.1",
        ref_kind="tag",
        vendor_dir="academic-research-skills",
        skills=(
            "academic-paper",
            "academic-paper-reviewer",
            "academic-pipeline",
            "deep-research",
        ),
        source_lines=(),
        short_old="v3.21.2",
        short_new="v3.23.1",
        full_old="8fa3d651ad45da9e02762a6ba1fa3d1f231f91b6",
    ),
    Pack(
        id="officecli",
        repo="https://github.com/iOfficeAI/OfficeCLI.git",
        ref="v1.0.158",
        ref_kind="tag",
        vendor_dir="OfficeCli",
        skills=(
            "officecli",
            "officecli-academic-paper",
            "officecli-data-dashboard",
            "officecli-docx",
            "officecli-financial-model",
            "officecli-pitch-deck",
            "officecli-pptx",
            "officecli-word-form",
            "officecli-xlsx",
            "morph-ppt",
            "morph-ppt-3d",
        ),
        source_lines=(),
        short_old="v1.0.149",
        short_new="v1.0.158",
        full_old="6e83afe02750af15521e09aa4ed941ccba71b94e",
        old_date="2026-09-10",
        new_date="2026-10-20",
    ),
)


def run(args: list[str], cwd: Path | None = None) -> str:
    result = subprocess.run(
        args,
        cwd=cwd,
        check=True,
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
    )
    return result.stdout.strip()


def clone(pack: Pack, dest: Path) -> str:
    if pack.ref_kind == "tag":
        run(["git", "clone", "--depth", "1", "--branch", pack.ref, pack.repo, str(dest)])
    else:
        run(["git", "init", str(dest)])
        run(["git", "-C", str(dest), "remote", "add", "origin", pack.repo])
        run(["git", "-C", str(dest), "fetch", "--depth", "1", "origin", pack.ref])
        run(["git", "-C", str(dest), "checkout", "--detach", "FETCH_HEAD"])
    return run(["git", "-C", str(dest), "rev-parse", "HEAD"])


def find_skill_root(repo: Path, names: tuple[str, ...]) -> Path:
    if any((repo / name).is_dir() for name in names):
        return repo
    nested = repo / "skills"
    if nested.is_dir() and any((nested / name).is_dir() for name in names):
        return nested
    raise SystemExit(f"none of {names} exists under {repo} or {repo / 'skills'}")


def text_files(root: Path):
    for path in root.rglob("*"):
        if not path.is_file():
            continue
        if path.suffix in {".md", ".json", ".yaml", ".yml"}:
            yield path


def rewrite_shared_refs(text: str, replacement: str) -> str:
    needle = "shared/"
    out: list[str] = []
    rest = text
    while True:
        idx = rest.find(needle)
        if idx < 0:
            out.append(rest)
            return "".join(out)
        prev = rest[idx - 1] if idx else ""
        blocked = bool(prev) and (prev.isascii() and prev.isalnum() or prev in "_-./")
        out.append(rest[:idx])
        out.append(needle if blocked else replacement)
        rest = rest[idx + len(needle):]


def rewrite_wisp_frontmatter(skill_md: Path) -> None:
    if not skill_md.is_file():
        return
    text = skill_md.read_text(encoding="utf-8")
    match = re.match(r"^---\r?\n(.*?)\n---", text, re.S)
    if not match:
        return
    frontmatter = match.group(1)
    if re.search(r"^\s*superscience:", frontmatter, re.M):
        return
    replaced, count = re.subn(
        r"^(\s*)wisp:(?=\s|\{|$)", r"\1superscience:", frontmatter, flags=re.M
    )
    if count:
        skill_md.write_text(
            text[: match.start(1)] + replaced + text[match.end(1):], encoding="utf-8"
        )


def ensure_frontmatter_name(skill_md: Path, expected: str) -> None:
    if not skill_md.is_file():
        return
    text = skill_md.read_text(encoding="utf-8")
    match = re.match(r"^---\r?\n(.*?)\n---", text, re.S)
    if not match:
        return
    frontmatter = match.group(1)
    replaced, count = re.subn(r"^name:.*$", f"name: {expected}", frontmatter, count=1, flags=re.M)
    if count:
        skill_md.write_text(
            text[: match.start(1)] + replaced + text[match.end(1):], encoding="utf-8"
        )


def apply_patches(name: str, skill_dir: Path) -> None:
    rewrite_wisp_frontmatter(skill_dir / "SKILL.md")
    if name == "nature-proposal-writer":
        ensure_frontmatter_name(skill_dir / "SKILL.md", "nature-proposal-writer")


def lift_academic_shared(repo: Path) -> None:
    names = ("academic-paper", "academic-paper-reviewer", "academic-pipeline", "deep-research")
    donor = repo / "shared"
    if not donor.is_dir():
        donor = next(
            (SKILLS / name / "shared" for name in names if (SKILLS / name / "shared").is_dir()),
            None,
        )
    if donor is None:
        return

    dest = SKILLS / "academic-shared"
    if dest.exists():
        shutil.rmtree(dest)
    shutil.copytree(donor, dest, symlinks=True)

    # Inside the shared package, references to `shared/x` become `x`.
    for path in text_files(dest):
        text = path.read_text(encoding="utf-8")
        rewritten = rewrite_shared_refs(text, "")
        if rewritten != text:
            path.write_text(rewritten, encoding="utf-8")
    (dest / "SKILL.md").write_text(ACADEMIC_SHARED_SKILL_MD, encoding="utf-8")

    for name in names:
        skill_dir = SKILLS / name
        if not skill_dir.is_dir():
            continue
        for path in text_files(skill_dir):
            if "shared" in path.relative_to(skill_dir).parts[:-1]:
                continue
            text = path.read_text(encoding="utf-8")
            rewritten = rewrite_shared_refs(text, "../academic-shared/")
            if rewritten != text:
                path.write_text(rewritten, encoding="utf-8")
        nested = skill_dir / "shared"
        if nested.is_dir():
            shutil.rmtree(nested)


def replace_bundled_pin(pack_id: str, pin: str) -> None:
    text = UPDATE_RS.read_text(encoding="utf-8")
    pattern = re.compile(
        r'(id:\s*"' + re.escape(pack_id) + r'",.*?bundled_pin:\s*")([^"]*)(")',
        re.S,
    )
    replaced, count = pattern.subn(r"\g<1>" + pin + r"\g<3>", text, count=1)
    if count != 1:
        raise SystemExit(f"could not update bundled_pin for {pack_id}")
    UPDATE_RS.write_text(replaced, encoding="utf-8")


def update_metadata(pack: Pack, sha: str, date: str) -> None:
    source = SKILLS / "_vendor" / pack.vendor_dir / "SOURCE.txt"
    if pack.id == "nature-skills":
        body = f"""Source: https://github.com/Yuan1z0825/nature-skills
Commit: {sha} ({date})
License: Apache-2.0 (see LICENSE)
Vendored into skills/nature-* for SuperScience/天成科研助手 bundling.

Local adaptations (do not drop on refresh):
- nature-proposal-writer/SKILL.md frontmatter name stays nature-proposal-writer
  (upstream uses researchwrite; SuperScience discovery requires name == directory).
"""
        pin = sha
    elif pack.id == "academic-research-skills":
        body = f"""Source: https://github.com/Imbad0202/academic-research-skills
Version: {pack.ref} ({sha})
License: CC BY-NC 4.0 (see LICENSE)
Vendored: academic-paper, academic-paper-reviewer, academic-pipeline, deep-research, academic-shared
Note: upstream v3.21+ ships a single repo-root shared/; SuperScience lifts it to skills/academic-shared.
Older archives that still embed shared/ inside each skill are normalized the same way.
"""
        pin = pack.ref
    else:
        body = f"""Source: https://github.com/iOfficeAI/OfficeCLI
Tag: {pack.ref}
Commit: {sha} ({date})
License: Apache-2.0 (see LICENSE / NOTICE)
Vendored skill dirs: officecli*, morph-ppt*
(skills/officecli/LICENSE and NOTICE are copies of the repo-root license files).

Auto-update: allowlisted as pack id `officecli` (LatestSemverTag).
"""
        pin = pack.ref
    source.write_text(body, encoding="utf-8")
    replace_bundled_pin(pack.id, pin)

    # Keep the humanizer pin in sync with the content we already vendored.
    if pack.id == "nature-skills":
        humanizer = "f4518a8eab97b8bfebc66a89d34320a89bef6930"
        text = UPDATE_RS.read_text(encoding="utf-8")
        text = text.replace("91f3d394db8419c20d67ebe22a96cf8fee0a404b", humanizer)
        UPDATE_RS.write_text(text, encoding="utf-8")

    replacements = {
        pack.short_old: pack.short_new,
        pack.full_old: pack.full_new or sha,
    }
    for doc in (SKILLS / "_vendor" / "README.md", SKILLS / "THIRD_PARTY_LICENSES.md"):
        text = doc.read_text(encoding="utf-8")
        for old, new in replacements.items():
            if old:
                text = text.replace(old, new)
        if pack.old_date and pack.new_date:
            # Only replace the date adjacent to this pack's pin.
            text = re.sub(
                rf"({re.escape(pack.full_new or sha)}[^\n]*?){re.escape(pack.old_date)}",
                rf"\g<1>{date}",
                text,
            )
        doc.write_text(text, encoding="utf-8")


def main() -> int:
    if not SKILLS.is_dir():
        raise SystemExit(f"missing skills directory: {SKILLS}")

    with tempfile.TemporaryDirectory(prefix="vendor-skills-") as temp:
        temp_root = Path(temp)
        for pack in PACKS:
            print(f"==> {pack.id} ({pack.ref})", flush=True)
            repo = temp_root / pack.id
            sha = clone(pack, repo)
            date = run(["git", "-C", str(repo), "log", "-1", "--format=%cs"])
            search = find_skill_root(repo, pack.skills)

            for name in pack.skills:
                source = search / name
                if not source.is_dir():
                    raise SystemExit(f"{pack.id}: missing skill directory {source}")
                dest = SKILLS / name
                if dest.exists():
                    shutil.rmtree(dest)
                shutil.copytree(source, dest, symlinks=True)
                apply_patches(name, dest)

            if pack.id == "academic-research-skills":
                lift_academic_shared(repo)
            if pack.id == "officecli":
                for name in ("LICENSE", "NOTICE"):
                    source = repo / name
                    if source.is_file():
                        shutil.copy2(source, SKILLS / "officecli" / name)

            update_metadata(pack, sha, date)
            print(f"    {sha} ({date})", flush=True)

    print("done")
    return 0


if __name__ == "__main__":
    sys.exit(main())
