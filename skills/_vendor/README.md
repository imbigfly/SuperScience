# Vendored third-party skill sources

Bundled into SuperScience `skills/` for offline discovery and DMG packaging
(`src-tauri/tauri.conf.json` maps `../skills/` → app `skills/`).

| Source | Upstream | What we ship |
|---|---|---|
| Academic research skills | https://github.com/Imbad0202/academic-research-skills v3.20.1 (CC BY-NC 4.0) | `academic-paper`, `academic-paper-reviewer`, `academic-pipeline`, `deep-research`, plus lifted `academic-shared` |
| nature-skills | https://github.com/Yuan1z0825/nature-skills `c171989d` (Apache-2.0) | `nature-*` including `nature-shared` |
| Humanizer-zh | https://github.com/op7418/Humanizer-zh (MIT) | `humanizer-zh` |
| academic-search-pro | https://skillhub.cn/skills/user_8d26dabd/academic-search-pro v1.0.5 (no LICENSE in package) | `academic-search-pro` |

Do not place a `SKILL.md` directly under `_vendor/` — discovery walks `skills/*/SKILL.md`.
Keep `_vendor/*/README.md` out of tree if upstream docs mention legacy host SDKs (scan tests forbid those strings under `skills/`).
