# Skills

SuperScience discovers `SKILL.md` packages from several scopes. The Skills settings
page shows the scope and absolute source path for every discovered skill, and
the Agent's `search_skills` result includes the same `scope` and `path` fields.

Discovery uses this precedence when two packages declare the same public name:

1. `bundled` — the read-only catalog shipped with SuperScience.
2. `project` — `<project>/.superscience/skills` for workflows owned by one project.
3. `global` — `~/.superscience/skills` for workflows shared by all projects.
4. `extra` — directories configured through `SUPERSCIENCE_SKILLS_PATH`, in configured
   order.
5. `plugin` — Skills from enabled feature plugins. A plugin never replaces a
   host Skill with the same name.

**Settings → Skills → Reload skills** rescans all of these locations without
restarting SuperScience. Newly discovered Skills are enabled by default. Existing
Skills that the user explicitly disabled remain disabled. Idle conversation
Agents are rebuilt on their next turn, so the new index is used without losing
conversation history or restarting the persistent Python/R runtime.

The **Add skill** action installs or updates a global Skill. A project Skill can
be managed with the project files under `.superscience/skills` and then loaded with
**Reload skills**. Only global Skills can be deleted from the Skills settings
page; project and extra-path files remain owned by their project or source
directory. Plugin Skills are managed from their plugin card.

Tags declared in `SKILL.md` appear automatically. Tags edited in Settings are a
user override and are also applied to Agent `search_skills` queries after the
next idle-Agent rebuild.
