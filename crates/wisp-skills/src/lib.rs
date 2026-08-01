//! SKILL.md discovery + the `use_skill` tool.

pub mod index;
pub mod manifest;
pub mod portfolio;
pub mod tool;

pub use index::{
    bundled_dir, list_resources, parse_skill_file, Skill, SkillCatalogAudit, SkillCatalogRecord,
    SkillCatalogSourceAudit, SkillIndex, SkillSource,
};
pub use manifest::{SkillManifest, SkillSideEffects, SkillTags, WispSkillMetadata};
pub use portfolio::{
    plan_portfolio, PortfolioCandidate, PortfolioConfig, PortfolioDeferral, PortfolioPlan,
    PortfolioSelection, PortfolioTier, ResearchIntent,
};
pub use tool::{render_skill, ListSkillCatalogTool, SearchSkillsTool, UseSkillTool};
