//! SKILL.md discovery + the `use_skill` tool.

pub mod index;
pub mod tool;

pub use index::{
    bundled_dir, list_resources, parse_skill_file, Skill, SkillCatalogAudit, SkillCatalogRecord,
    SkillCatalogSourceAudit, SkillIndex, SkillSource,
};
pub use tool::{render_skill, ListSkillCatalogTool, SearchSkillsTool, UseSkillTool};
