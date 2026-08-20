//! SKILL.md discovery + the `use_skill` tool.

pub mod index;
pub mod manifest;
pub mod tool;
pub mod update;

pub use index::{
    bundled_dir, extra_skill_dirs_from_env, list_resources, parse_skill_file, skill_source_paths,
    Skill, SkillCatalogAudit, SkillCatalogRecord, SkillCatalogSourceAudit, SkillIndex, SkillSource,
};
pub use manifest::{SkillManifest, SkillSideEffects, SkillTags, SuperscienceSkillMetadata};
pub use tool::{render_skill, ListSkillCatalogTool, SearchSkillsTool, UseSkillTool};
