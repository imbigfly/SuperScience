# Vendored third-party skill sources

Bundled into SuperScience `skills/` for offline discovery and DMG packaging
(`src-tauri/tauri.conf.json` maps `../skills/` → app `skills/`).

| Source | Upstream | What we ship |
|---|---|---|
| Academic research skills | https://github.com/Imbad0202/academic-research-skills v3.21.2 (CC BY-NC 4.0) | `academic-paper`, `academic-paper-reviewer`, `academic-pipeline`, `deep-research`, plus lifted `academic-shared` |
| nature-skills | https://github.com/Yuan1z0825/nature-skills `b1a37a2b` (Apache-2.0) | `nature-*` including `nature-shared` and `nature-image2ppt` |
| Humanizer-zh | https://github.com/op7418/Humanizer-zh `91f3d394` (MIT) | `humanizer-zh` |
| academic-search-pro | https://skillhub.cn/skills/user_8d26dabd/academic-search-pro v1.0.5 (no LICENSE in package) | `academic-search-pro` |

Do not place a `SKILL.md` directly under `_vendor/` — discovery walks `skills/*/SKILL.md`.
Keep `_vendor/*/README.md` out of tree if upstream docs mention legacy host SDKs (scan tests forbid those strings under `skills/`).
