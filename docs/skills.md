# Skills

SuperScience discovers `SKILL.md` packages from several scopes. The Skills settings
page shows the scope and absolute source path for every discovered skill, and
the Agent's `search_skills` result includes the same `scope` and `path` fields.
For inventory questions, `list_skill_catalog` pages through the complete
discovered or effective view and reports separate discovered, effective,
shadowed, parse-error, and currently searchable enabled counts. Search result
counts must not be interpreted as the configured Skill inventory. The
`current_configured_enabled_count` field is the authoritative count for the
current Agent snapshot. If a user-provided or remembered UI count differs, the
Agent reports the discrepancy instead of inventing an installed/enabled
distinction.

Skills may declare an optional `superscience` YAML mapping with `schema_version: 1`
and controlled `domains`, `research_stages`, `roles`, `evidence_types`,
`outputs`, and `side_effects`. Other YAML keys remain valid extra fields; a
`wisp:` key is not planner metadata. Invalid SuperScience
semantics are retained as catalog parse-error records instead of silently
entering the effective catalog.

Discovery uses this precedence when two packages declare the same public name
(first path wins):

1. `project` — `<project>/.superscience/skills` for
   workflows owned by one project.
2. `global` — `~/.superscience/skills` for workflows
   shared by all projects. Skill auto-update and **Add skill** write here.
3. `extra` — directories from `SUPERSCIENCE_SKILLS_PATH`,
   in configured order.
4. `bundled` — the read-only catalog shipped with SuperScience.
5. `plugin` — Skills from enabled feature plugins. A plugin never replaces a
   host Skill with the same name.

**Settings → Skills → Reload skills** rescans all of these locations without
restarting SuperScience. Newly discovered Skills are enabled by default. Existing
Skills that the user explicitly disabled remain disabled. Idle conversation
Agents are rebuilt on their next turn, so the new index is used without losing
conversation history or restarting the persistent Python/R runtime.

## Skill auto-update

**Settings → Skills** includes **Skill auto-update** (default on). When enabled,
startup and **Check and update now** query allowlisted GitHub sources
(`nature-skills`, `academic-research-skills`, `humanizer-zh`, and an already
installed `ppt-master`) and install newer packs into `~/.superscience/skills`.
The signed bundled tree is never rewritten. Failures are logged; the UI stays
quiet on startup. Semver overlays that fall behind a newer bundled pin are
dropped so an app upgrade is not pinned by a stale overlay. Academic research
skills share one `academic-shared` package; auto-update still downloads the
upstream archive (which embeds `shared/` in each skill) and then lifts it.
Agents should author project Skills under `<project>/.superscience/skills`, not
rewrite the global auto-update overlay directory.

The **Add skill** action installs or updates a global Skill from a `SKILL.md`
file, a Skill folder, or a ZIP archive. A ZIP may contain `SKILL.md` directly or
wrap one Skill in a single top-level folder. A project Skill can be managed with
the project files under `.superscience/skills` and then loaded with **Reload skills**.
Only global Skills can be deleted from the Skills settings page; project and
extra-path files remain owned by their project or source directory. Plugin
Skills are managed from their plugin card.

Tags declared in `SKILL.md` appear automatically. Tags edited in Settings are a
user override and are also applied to Agent `search_skills` queries after the
next idle-Agent rebuild. Search matches name, description, and tags.

`search_skills` normalizes case and common separators before matching names,
descriptions, and tags. Continuous CJK queries also contribute bounded 2–4
character terms, so ordinary Chinese task descriptions do not have to contain
spaces to find matching Chinese metadata. Agent guidance asks for one retry
with cross-language domain synonyms when the first query has no confident
match. Search is still local lexical retrieval; it does not call an embedding
service or send the Skill catalog to a third party.

The **Capabilities** summary uses the same current enabled Skill inventory and
splits it into bundled and project-added counts. Project-added includes project,
global, extra-path, and project-enabled plugin Skills. MCP counts are split into
bundled packages and enabled custom/plugin services available to the project.
