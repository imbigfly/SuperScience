# Vendored third-party skill sources

Bundled into SuperScience `skills/` for offline discovery and DMG packaging
(`src-tauri/tauri.conf.json` maps `../skills/` → app `skills/`).

| Source | Upstream | What we ship |
|---|---|---|
| Academic research skills | https://github.com/Imbad0202/academic-research-skills v3.20.0 (CC BY-NC 4.0) | `academic-paper`, `academic-paper-reviewer`, `academic-pipeline`, `deep-research` (each embeds `shared/`) |
| Humanizer-zh | https://github.com/op7418/Humanizer-zh (MIT) | `humanizer-zh` |

Do not place a `SKILL.md` directly under `_vendor/` — discovery walks `skills/*/SKILL.md`.
Keep `_vendor/*/README.md` out of tree if upstream docs mention legacy host SDKs (scan tests forbid those strings under `skills/`).
